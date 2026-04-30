import { useState } from 'react';
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
  labels: { name: string; base_url: string; api_key: string; model: string };
}

export default function ProviderTabs({ tabs, labels }: Props) {
  const [active, setActive] = useState(tabs[0].id);
  const cur = tabs.find((t) => t.id === active) ?? tabs[0];

  const fields: Array<[string, string]> = [
    [labels.name, `${cur.name} (${cur.tag})`],
    [labels.base_url, cur.baseUrl],
    [labels.api_key, cur.apiKey],
    [labels.model, cur.model],
  ];

  return (
    <div className="surface-card overflow-hidden">
      {/* Tabs row with animated underline */}
      <div className="relative flex items-center gap-1 border-b border-white/5 bg-white/[0.02] px-4 pt-3">
        {tabs.map((t) => (
          <button
            key={t.id}
            onClick={() => setActive(t.id)}
            className={
              'relative px-3 py-2.5 text-[12.5px] font-medium transition-colors ' +
              (active === t.id ? 'text-white' : 'text-zinc-500 hover:text-zinc-300')
            }
          >
            <span className="inline-flex items-center gap-1.5">
              <span
                className={
                  'h-1.5 w-1.5 rounded-full transition-colors ' +
                  (active === t.id ? 'bg-brand-400' : 'bg-zinc-700')
                }
              />
              {t.name}
              <span className="text-[10px] uppercase tracking-wider text-zinc-500">{t.tag}</span>
            </span>
            {active === t.id && (
              <motion.span
                layoutId="provider-tab-underline"
                className="absolute -bottom-px left-0 right-0 h-px bg-brand-400"
                transition={{ type: 'spring', stiffness: 380, damping: 30 }}
              />
            )}
          </button>
        ))}
        <span className="ml-auto rounded-full bg-accent-lime/10 px-2 py-0.5 font-mono text-[10px] text-accent-lime ring-1 ring-accent-lime/20">
          ● connected
        </span>
      </div>

      {/* Fields */}
      <div className="p-6">
        <AnimatePresence mode="wait">
          <motion.div
            key={cur.id}
            initial={{ opacity: 0, y: 8, filter: 'blur(4px)' }}
            animate={{ opacity: 1, y: 0, filter: 'blur(0px)' }}
            exit={{ opacity: 0, y: -6, filter: 'blur(2px)' }}
            transition={{ duration: 0.35, ease: [0.22, 1, 0.36, 1] }}
            className="grid gap-4 sm:grid-cols-2"
          >
            {fields.map(([k, v], i) => (
              <div key={k} className={i >= 1 ? 'sm:col-span-2' : ''}>
                <p className="mb-1.5 text-[11px] font-medium uppercase tracking-wider text-zinc-500">{k}</p>
                <div className={
                  'rounded-lg border border-white/5 bg-ink-950/60 px-3 py-2.5 text-sm text-zinc-200 ' +
                  (i === 0 ? '' : 'font-mono text-[12.5px]')
                }>
                  {v}
                </div>
              </div>
            ))}
          </motion.div>
        </AnimatePresence>
      </div>
    </div>
  );
}
