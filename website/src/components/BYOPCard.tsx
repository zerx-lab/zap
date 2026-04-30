import { useEffect, useState } from 'react';
import { motion, AnimatePresence } from 'framer-motion';

type Tab = {
  id: string;
  name: string;
  tag: string;
  baseUrl: string;
  apiKey: string;
  model: string;
};

interface Props {
  tabs: Tab[];
  hint: string;
}

export default function BYOPCard({ tabs, hint }: Props) {
  const [i, setI] = useState(0);
  useEffect(() => {
    const t = setInterval(() => setI((x) => (x + 1) % tabs.length), 3800);
    return () => clearInterval(t);
  }, [tabs.length]);

  const cur = tabs[i];

  return (
    <div className="relative flex h-full flex-col">
      <div className="flex items-center gap-1.5">
        {tabs.map((t, idx) => (
          <button
            key={t.id}
            onClick={() => setI(idx)}
            className={
              'group relative inline-flex items-center gap-1.5 rounded-md px-2.5 py-1 text-[11px] font-medium transition ' +
              (idx === i
                ? 'bg-white/[0.06] text-zinc-100'
                : 'text-zinc-500 hover:text-zinc-300')
            }
          >
            <span
              className={
                'h-1.5 w-1.5 rounded-full ' +
                (idx === i ? 'bg-brand-400' : 'bg-zinc-700')
              }
            />
            {t.name}
          </button>
        ))}
        <span className="ml-auto hidden font-mono text-[10px] text-zinc-600 sm:block">{hint}</span>
      </div>

      <div className="relative mt-4 flex-1">
        <AnimatePresence mode="wait">
          <motion.div
            key={cur.id}
            initial={{ opacity: 0, y: 12, filter: 'blur(6px)' }}
            animate={{ opacity: 1, y: 0, filter: 'blur(0px)' }}
            exit={{ opacity: 0, y: -8, filter: 'blur(4px)' }}
            transition={{ duration: 0.45, ease: [0.22, 1, 0.36, 1] }}
            className="flex h-full flex-col"
          >
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-2">
                <span className="font-display text-lg font-semibold tracking-tight text-white">{cur.name}</span>
                <span className="rounded-full border border-white/10 bg-white/[0.04] px-2 py-0.5 text-[10px] uppercase tracking-wider text-zinc-400">{cur.tag}</span>
              </div>
              <span className="font-mono text-[10.5px] text-zinc-500">~/.config/openwarp.toml</span>
            </div>

            <div className="mt-4 grid flex-1 grid-cols-1 gap-2.5 sm:grid-cols-[auto_1fr]">
              {[
                ['base_url', cur.baseUrl],
                ['api_key', cur.apiKey],
                ['model', cur.model],
              ].map(([k, v]) => (
                <FieldRow key={k} k={k as string} v={v as string} />
              ))}
            </div>

            <div className="mt-4 flex items-center justify-between border-t border-white/5 pt-3 font-mono text-[10.5px] text-zinc-500">
              <div className="flex items-center gap-2">
                <span className="relative inline-flex h-1.5 w-1.5">
                  <span className="absolute inset-0 rounded-full bg-accent-lime opacity-70 animate-ping" />
                  <span className="relative h-1.5 w-1.5 rounded-full bg-accent-lime" />
                </span>
                connected · streaming ok
              </div>
              <div>POST → /chat/completions</div>
            </div>
          </motion.div>
        </AnimatePresence>
      </div>
    </div>
  );
}

function FieldRow({ k, v }: { k: string; v: string }) {
  return (
    <>
      <div className="font-mono text-[11px] text-zinc-500 sm:py-1">{k}</div>
      <div className="rounded-md border border-white/5 bg-ink-950/70 px-2.5 py-1.5 font-mono text-[12px] text-zinc-200">
        {v}
      </div>
    </>
  );
}
