import { useState } from 'react';

interface Props {
  command: string;
  copyLabel: string;
  copiedLabel: string;
}

export default function CommandCard({ command, copyLabel, copiedLabel }: Props) {
  const [copied, setCopied] = useState(false);
  const onCopy = async () => {
    try {
      await navigator.clipboard.writeText(command);
      setCopied(true);
      setTimeout(() => setCopied(false), 1800);
    } catch {
      /* ignored */
    }
  };

  return (
    <button
      onClick={onCopy}
      className="group flex w-full items-center gap-3 rounded-lg border border-white/10 bg-ink-950/80 px-3 py-2.5 text-left font-mono text-[12.5px] text-zinc-200 transition hover:border-white/20 hover:bg-ink-950"
      aria-label="copy command"
    >
      <span className="select-none text-brand-300">$</span>
      <span className="min-w-0 flex-1 truncate">{command}</span>
      <span
        className={
          'inline-flex items-center gap-1 rounded-md px-2 py-0.5 text-[10.5px] uppercase tracking-wider transition ' +
          (copied
            ? 'bg-accent-lime/10 text-accent-lime ring-1 ring-accent-lime/20'
            : 'border border-white/10 text-zinc-400 group-hover:text-zinc-200')
        }
      >
        {copied ? (
          <>
            <svg className="h-3 w-3" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="3" strokeLinecap="round" strokeLinejoin="round"><path d="m5 12 5 5L20 7"/></svg>
            {copiedLabel}
          </>
        ) : (
          <>
            <svg className="h-3 w-3" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><rect x="9" y="9" width="11" height="11" rx="2"/><path d="M5 15V5a2 2 0 0 1 2-2h10"/></svg>
            {copyLabel}
          </>
        )}
      </span>
    </button>
  );
}
