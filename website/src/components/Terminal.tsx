import { useEffect, useMemo, useRef, useState } from 'react';
import { motion, AnimatePresence } from 'framer-motion';

export type Scenario = {
  model: string;
  tag: string;
  user: string;
  reply: string;
  suggest: string[];
};

interface Props {
  scenarios: Scenario[];
  tabs: string[];
  breadcrumb: string;
  status: {
    tokens: string;
    latency: string;
    local: string;
    streaming: string;
    ready: string;
  };
}

type Phase = 'prompt' | 'thinking' | 'streaming' | 'done';

export default function Terminal({ scenarios, tabs, breadcrumb, status }: Props) {
  const [idx, setIdx] = useState(0);
  const [phase, setPhase] = useState<Phase>('prompt');
  const [tokens, setTokens] = useState(0);
  const [latency, setLatency] = useState(0);
  const cancelRef = useRef(false);

  const current = scenarios[idx];
  const words = useMemo(() => current.reply.split(/(\s+)/), [current.reply]);

  // Drive the lifecycle of one scenario
  useEffect(() => {
    cancelRef.current = false;
    setTokens(0);
    setLatency(0);
    setPhase('prompt');

    const timers: ReturnType<typeof setTimeout>[] = [];
    timers.push(setTimeout(() => !cancelRef.current && setPhase('thinking'), 700));
    timers.push(setTimeout(() => !cancelRef.current && setPhase('streaming'), 1500));
    // approximate stream length: ~30ms per word
    const streamDuration = Math.max(1600, words.length * 55);
    timers.push(setTimeout(() => !cancelRef.current && setPhase('done'), 1500 + streamDuration));
    timers.push(
      setTimeout(() => {
        if (cancelRef.current) return;
        setIdx((i) => (i + 1) % scenarios.length);
      }, 1500 + streamDuration + 2400),
    );

    return () => {
      cancelRef.current = true;
      timers.forEach(clearTimeout);
    };
  }, [idx, words.length, scenarios.length]);

  // Live status counters
  useEffect(() => {
    if (phase !== 'streaming') return;
    const tick = setInterval(() => {
      setTokens((t) => t + Math.floor(Math.random() * 6) + 3);
      setLatency((l) => (l < 420 ? l + Math.floor(Math.random() * 18) + 6 : l));
    }, 90);
    return () => clearInterval(tick);
  }, [phase]);

  return (
    <div className="relative">
      {/* Floating "model swap" hint */}
      <AnimatePresence>
        <motion.div
          key={current.model}
          initial={{ opacity: 0, y: -8, scale: 0.96 }}
          animate={{ opacity: 1, y: 0, scale: 1 }}
          exit={{ opacity: 0, y: 8, scale: 0.96 }}
          transition={{ type: 'spring', stiffness: 220, damping: 20 }}
          className="absolute -top-3 left-6 z-10 inline-flex items-center gap-2 rounded-full border border-white/10 bg-ink-900/90 px-3 py-1 text-[11px] font-medium text-zinc-200 shadow-card backdrop-blur-md"
        >
          <span className="relative inline-flex h-1.5 w-1.5">
            <span className="absolute inset-0 rounded-full bg-brand-400 opacity-70 animate-ping" />
            <span className="relative h-1.5 w-1.5 rounded-full bg-brand-400" />
          </span>
          <span className="font-mono">{current.model}</span>
          <span className="text-zinc-500">·</span>
          <span className="text-zinc-400">{current.tag}</span>
        </motion.div>
      </AnimatePresence>

      <div className="surface-card relative overflow-hidden">
        {/* Top chrome: traffic + tabs + breadcrumb */}
        <div className="flex items-center justify-between gap-3 border-b border-white/5 bg-white/[0.02] px-4 py-2.5">
          <div className="flex items-center gap-1.5">
            <span className="h-3 w-3 rounded-full bg-[#ff5f57]" />
            <span className="h-3 w-3 rounded-full bg-[#febc2e]" />
            <span className="h-3 w-3 rounded-full bg-[#28c840]" />
          </div>
          <div className="flex flex-1 items-center gap-1 overflow-x-auto px-2 [scrollbar-width:none] [&::-webkit-scrollbar]:hidden">
            {tabs.map((tab, i) => (
              <button
                key={tab}
                className={
                  'group relative inline-flex flex-none items-center gap-1.5 rounded-md px-2.5 py-1 text-[11px] transition ' +
                  (i === 1
                    ? 'bg-white/[0.06] text-zinc-100'
                    : 'text-zinc-500 hover:text-zinc-300')
                }
              >
                <span
                  className={
                    'h-1.5 w-1.5 rounded-full ' +
                    (i === 1 ? 'bg-accent-lime' : 'bg-zinc-700')
                  }
                />
                {tab}
              </button>
            ))}
          </div>
          <div className="hidden flex-none font-mono text-[11px] text-zinc-500 sm:block">{breadcrumb}</div>
        </div>

        {/* Body */}
        <div className="relative px-5 py-5 font-mono text-[13.5px] leading-relaxed">
          {/* Scene swap with crossfade */}
          <AnimatePresence mode="wait">
            <motion.div
              key={idx}
              initial={{ opacity: 0, y: 10 }}
              animate={{ opacity: 1, y: 0 }}
              exit={{ opacity: 0, y: -8, transition: { duration: 0.25 } }}
              transition={{ duration: 0.35, ease: [0.22, 1, 0.36, 1] }}
              className="space-y-3"
            >
              {/* User prompt */}
              <div className="flex items-start gap-2">
                <span className="select-none text-brand-400">❯</span>
                <span className="text-zinc-100">{current.user}</span>
              </div>

              {/* Thinking dots */}
              <AnimatePresence>
                {phase === 'thinking' && (
                  <motion.div
                    initial={{ opacity: 0 }}
                    animate={{ opacity: 1 }}
                    exit={{ opacity: 0 }}
                    className="flex items-center gap-1.5 pl-4 text-zinc-500"
                  >
                    {[0, 1, 2].map((i) => (
                      <motion.span
                        key={i}
                        className="h-1.5 w-1.5 rounded-full bg-zinc-500"
                        animate={{ opacity: [0.25, 1, 0.25], y: [0, -2, 0] }}
                        transition={{
                          duration: 1.1,
                          repeat: Infinity,
                          delay: i * 0.15,
                          ease: 'easeInOut',
                        }}
                      />
                    ))}
                  </motion.div>
                )}
              </AnimatePresence>

              {/* Streaming reply (word-level reveal with subtle blur-in) */}
              {(phase === 'streaming' || phase === 'done') && (
                <div className="pl-4 text-zinc-300">
                  {words.map((w, i) => (
                    <motion.span
                      key={i}
                      initial={{ opacity: 0, filter: 'blur(6px)', y: 4 }}
                      animate={{ opacity: 1, filter: 'blur(0px)', y: 0 }}
                      transition={{
                        duration: 0.35,
                        delay: i * 0.045,
                        ease: [0.22, 1, 0.36, 1],
                      }}
                    >
                      {w}
                    </motion.span>
                  ))}
                  {phase === 'streaming' && (
                    <motion.span
                      animate={{ opacity: [0.2, 1, 0.2] }}
                      transition={{ duration: 0.9, repeat: Infinity }}
                      className="ml-1 inline-block h-[1em] w-[2px] translate-y-[3px] bg-brand-400"
                    />
                  )}
                </div>
              )}

              {/* Code suggestion */}
              {phase === 'done' && (
                <motion.div
                  initial={{ opacity: 0, y: 6 }}
                  animate={{ opacity: 1, y: 0 }}
                  transition={{ duration: 0.4, delay: 0.1 }}
                  className="ml-4 mt-3 space-y-1 rounded-lg border border-white/5 bg-ink-950/70 p-3"
                >
                  {current.suggest.map((line, i) => (
                    <motion.div
                      key={i}
                      initial={{ opacity: 0, x: -4 }}
                      animate={{ opacity: 1, x: 0 }}
                      transition={{ delay: 0.18 + i * 0.08 }}
                      className="flex items-start gap-2 text-[12.5px]"
                    >
                      <span className="select-none text-zinc-600">{i + 1}</span>
                      <code
                        className={
                          line.trim().startsWith('//') || line.trim().startsWith('#')
                            ? 'text-zinc-500'
                            : 'text-zinc-200'
                        }
                      >
                        {line}
                      </code>
                    </motion.div>
                  ))}
                </motion.div>
              )}
            </motion.div>
          </AnimatePresence>
        </div>

        {/* Status bar */}
        <div className="flex flex-wrap items-center justify-between gap-x-3 gap-y-1 border-t border-white/5 bg-white/[0.015] px-3 py-2 font-mono text-[10.5px] text-zinc-500 sm:gap-2 sm:px-4">
          <div className="flex min-w-0 items-center gap-2 sm:gap-3">
            <span className="inline-flex items-center gap-1.5">
              <span
                className={
                  'h-1.5 w-1.5 rounded-full ' +
                  (phase === 'streaming'
                    ? 'bg-brand-400 animate-pulse'
                    : phase === 'thinking'
                      ? 'bg-amber-400'
                      : 'bg-accent-lime')
                }
              />
              {phase === 'streaming'
                ? status.streaming
                : phase === 'thinking'
                  ? '…'
                  : status.ready}
            </span>
            <span className="text-zinc-700">·</span>
            <span className="truncate text-zinc-400">{current.model}</span>
          </div>
          <div className="flex items-center gap-2 sm:gap-3">
            <span>
              <span className="text-zinc-400 tabular-nums">{tokens}</span>{' '}
              <span className="text-zinc-600">{status.tokens}</span>
            </span>
            <span className="text-zinc-700">·</span>
            <span>
              <span className="text-zinc-400 tabular-nums">{latency}</span>
              <span className="text-zinc-600">ms</span>
            </span>
            <span className="hidden text-zinc-700 sm:inline">·</span>
            <span className="hidden text-zinc-500 sm:inline">{status.local}</span>
          </div>
        </div>
      </div>

      {/* Scenario indicator dots */}
      <div className="mt-4 flex items-center justify-center gap-1.5">
        {scenarios.map((_, i) => (
          <button
            key={i}
            aria-label={`scenario ${i + 1}`}
            onClick={() => setIdx(i)}
            className="group h-1 overflow-hidden rounded-full bg-white/[0.06] transition-all"
            style={{ width: i === idx ? 28 : 8 }}
          >
            {i === idx && (
              <motion.span
                key={`bar-${idx}`}
                initial={{ width: '0%' }}
                animate={{ width: '100%' }}
                transition={{ duration: 5.4, ease: 'linear' }}
                className="block h-full bg-brand-400"
              />
            )}
          </button>
        ))}
      </div>
    </div>
  );
}
