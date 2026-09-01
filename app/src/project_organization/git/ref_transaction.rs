use std::{
    io::{self, BufRead, BufReader, BufWriter, Read, Write},
    path::Path,
    process::Stdio,
    thread,
};

/// Git 引用事务的当前协议阶段。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RefTransactionStage {
    Start,
    Prepare,
    Commit,
    Abort,
    Wait,
    ReadStderr,
}

/// Git 引用事务执行失败。
#[derive(Debug, thiserror::Error)]
pub enum RefTransactionError {
    #[error("git update-ref transaction cannot verify and delete the same ref `{full_ref}`")]
    ConflictingRefUpdate { full_ref: String },
    #[error("failed to start git update-ref transaction: {source}")]
    Start {
        #[source]
        source: io::Error,
    },
    #[error("git update-ref transaction is missing its {pipe} pipe")]
    MissingPipe { pipe: &'static str },
    #[error("failed to communicate with git update-ref during {stage:?}: {source}")]
    Io {
        stage: RefTransactionStage,
        #[source]
        source: io::Error,
    },
    #[error("git update-ref returned `{response}` during {stage:?}, expected `{expected}`")]
    UnexpectedResponse {
        stage: RefTransactionStage,
        expected: &'static str,
        response: String,
    },
    #[error("git update-ref exited during {stage:?}: {stderr}")]
    ProcessExited {
        stage: RefTransactionStage,
        stderr: String,
    },
    #[error("failed to join git update-ref stderr reader during {stage:?}")]
    StderrReaderPanicked { stage: RefTransactionStage },
}

/// 需要在引用事务中验证的引用。
pub(super) struct LockedRef<'a> {
    pub(super) full_ref: &'a str,
    pub(super) oid: &'a str,
}

/// 已完成 prepare、可在外部操作完成后提交或中止的引用删除事务。
#[derive(Debug)]
pub(crate) struct PreparedRefDelete {
    child: std::process::Child,
    stdin: Option<BufWriter<std::process::ChildStdin>>,
    stdout: BufReader<std::process::ChildStdout>,
    stderr_reader: Option<thread::JoinHandle<io::Result<Vec<u8>>>>,
    completed: bool,
}

impl PreparedRefDelete {
    /// 启动并 prepare 一个带可选合并目标验证的分支引用删除事务。
    pub(super) fn prepare(
        repository: &Path,
        branch_ref: &str,
        branch_oid: &str,
        merge_target: Option<LockedRef<'_>>,
    ) -> Result<Self, RefTransactionError> {
        let merge_target = merge_target.as_ref();
        if merge_target.is_some_and(|target| target.full_ref == branch_ref) {
            return Err(RefTransactionError::ConflictingRefUpdate {
                full_ref: branch_ref.to_string(),
            });
        }

        let mut child = command::blocking::Command::new("git")
            .arg("-C")
            .arg(repository)
            .args(["update-ref", "--stdin"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|source| RefTransactionError::Start { source })?;

        let stderr = child
            .stderr
            .take()
            .ok_or(RefTransactionError::MissingPipe { pipe: "stderr" })?;
        let stderr_reader = thread::spawn(move || {
            let mut stderr = stderr;
            let mut output = Vec::new();
            stderr.read_to_end(&mut output)?;
            Ok(output)
        });
        let stdin = child
            .stdin
            .take()
            .ok_or(RefTransactionError::MissingPipe { pipe: "stdin" })?;
        let stdout = child
            .stdout
            .take()
            .ok_or(RefTransactionError::MissingPipe { pipe: "stdout" })?;
        let mut transaction = Self {
            child,
            stdin: Some(BufWriter::new(stdin)),
            stdout: BufReader::new(stdout),
            stderr_reader: Some(stderr_reader),
            completed: false,
        };

        transaction.send_line(RefTransactionStage::Start, "start")?;
        transaction.expect_response(RefTransactionStage::Start, "start: ok")?;
        if let Some(target) = merge_target {
            transaction.send_line(
                RefTransactionStage::Prepare,
                &format!("verify {} {}", target.full_ref, target.oid),
            )?;
        }
        transaction.send_line(
            RefTransactionStage::Prepare,
            &format!("delete {branch_ref} {branch_oid}"),
        )?;
        transaction.send_line(RefTransactionStage::Prepare, "prepare")?;
        transaction.expect_response(RefTransactionStage::Prepare, "prepare: ok")?;

        Ok(transaction)
    }

    /// 提交已 prepare 的引用删除事务。
    pub(super) fn commit(mut self) -> Result<(), RefTransactionError> {
        self.send_line(RefTransactionStage::Commit, "commit")?;
        self.expect_response(RefTransactionStage::Commit, "commit: ok")?;
        self.finish_process(RefTransactionStage::Commit)
    }

    /// 中止已 prepare 的引用删除事务。
    pub(super) fn abort(mut self) -> Result<(), RefTransactionError> {
        self.send_line(RefTransactionStage::Abort, "abort")?;
        self.expect_response(RefTransactionStage::Abort, "abort: ok")?;
        self.finish_process(RefTransactionStage::Abort)
    }

    #[cfg(test)]
    pub(crate) fn terminate_for_test(&mut self) {
        let _ = self.child.kill();
    }

    fn send_line(
        &mut self,
        stage: RefTransactionStage,
        line: &str,
    ) -> Result<(), RefTransactionError> {
        let stdin = self.stdin.as_mut().ok_or_else(|| RefTransactionError::Io {
            stage,
            source: io::Error::new(
                io::ErrorKind::BrokenPipe,
                "git update-ref transaction stdin is closed",
            ),
        })?;
        stdin
            .write_all(line.as_bytes())
            .and_then(|()| stdin.write_all(b"\n"))
            .and_then(|()| stdin.flush())
            .map_err(|source| RefTransactionError::Io { stage, source })
    }

    fn expect_response(
        &mut self,
        stage: RefTransactionStage,
        expected: &'static str,
    ) -> Result<(), RefTransactionError> {
        let mut response = String::new();
        match self.stdout.read_line(&mut response) {
            Ok(0) => match self.finish_process(stage) {
                Ok(()) => Err(RefTransactionError::UnexpectedResponse {
                    stage,
                    expected,
                    response,
                }),
                Err(error) => Err(error),
            },
            Ok(_) => {
                let response = response.trim_end_matches(['\r', '\n']).to_string();
                if response == expected {
                    Ok(())
                } else {
                    Err(RefTransactionError::UnexpectedResponse {
                        stage,
                        expected,
                        response,
                    })
                }
            }
            Err(source) => Err(RefTransactionError::Io { stage, source }),
        }
    }

    fn finish_process(&mut self, stage: RefTransactionStage) -> Result<(), RefTransactionError> {
        if self.completed {
            return Ok(());
        }

        self.stdin.take();
        let status = self
            .child
            .wait()
            .map_err(|source| RefTransactionError::Io {
                stage: RefTransactionStage::Wait,
                source,
            })?;
        let stderr = match self.read_stderr() {
            Ok(stderr) => stderr,
            Err(error) => {
                self.completed = true;
                return Err(error);
            }
        };
        self.completed = true;

        if status.success() {
            Ok(())
        } else {
            Err(RefTransactionError::ProcessExited {
                stage,
                stderr: stderr_to_string(stderr),
            })
        }
    }

    fn read_stderr(&mut self) -> Result<Vec<u8>, RefTransactionError> {
        let Some(stderr_reader) = self.stderr_reader.take() else {
            return Ok(Vec::new());
        };
        match stderr_reader.join() {
            Ok(Ok(stderr)) => Ok(stderr),
            Ok(Err(source)) => Err(RefTransactionError::Io {
                stage: RefTransactionStage::ReadStderr,
                source,
            }),
            Err(_) => Err(RefTransactionError::StderrReaderPanicked {
                stage: RefTransactionStage::ReadStderr,
            }),
        }
    }
}

impl Drop for PreparedRefDelete {
    fn drop(&mut self) {
        if self.completed {
            return;
        }

        if self.stdin.is_some() {
            let _ = self.send_line(RefTransactionStage::Abort, "abort");
            let _ = self.expect_response(RefTransactionStage::Abort, "abort: ok");
        }
        let _ = self.finish_process(RefTransactionStage::Abort);
    }
}

fn stderr_to_string(stderr: Vec<u8>) -> String {
    String::from_utf8(stderr).unwrap_or_else(|error| {
        format!(
            "git update-ref stderr contained non-UTF-8 bytes: {:?}",
            error.into_bytes()
        )
    })
}

#[cfg(test)]
#[path = "ref_transaction_tests.rs"]
mod tests;
