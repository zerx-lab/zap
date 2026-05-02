import { useState } from 'react';
import { motion, AnimatePresence } from 'framer-motion';

type Tab = {
  id: string;
  name: string;
  tag: string;
  protocol: string;
  baseUrl: string;
  endpoint: string;
  apiKey: string;
  model: string;
};

interface Props {
  tabs: Tab[];
  labels: {
    name: string;
    protocol: string;
    base_url: string;
    endpoint: string;
    api_key: string;
    model: string;
  };
}

export default function ProviderTabs({ tabs, labels }: Props) {
  const [active, setActive] = useState(tabs[0].id);
  const cur = tabs.find((t) => t.id === active) ?? tabs[0];

  const fields: Array<{ k: string; v: string; mono?: boolean; full?: boolean }> = [
    { k: labels.name, v: `${cur.name} (${cur.tag})` },
    { k: labels.protocol, v: cur.protocol, mono: true },
    { k: labels.base_url, v: cur.baseUrl, mono: true, full: true },
    { k: labels.endpoint, v: cur.endpoint, mono: true, full: true },
    { k: labels.api_key, v: cur.apiKey, mono: true, full: true },
    { k: labels.model, v: cur.model, mono: true, full: true },
  ];

  return (
    <div className="surface-card overflow-hidden">
      {/* Tabs row with animated underline */}
      <div className="relative flex items-center gap-1 border-b border-white/5 bg-white/[0.02] px-2 pt-2 sm:px-4 sm:pt-3">
        <div className="-mb-px flex min-w-0 flex-1 items-center gap-1 overflow-x-auto whitespace-nowrap pb-px [scrollbar-width:none] [&::-webkit-scrollbar]:hidden">
          {tabs.map((t) => (
            <button
              key={t.id}
              onClick={() => setActive(t.id)}
              className={
                'relative flex-none px-2.5 py-2.5 text-[12.5px] font-medium transition-colors sm:px-3 ' +
                (active === t.id ? 'text-white' : 'text-zinc-500 hover:text-zinc-300')
              }
            >
              <span className="inline-flex items-center gap-1.5">
                <span
                  className={
                    'h-1.5 w-1.5 flex-none rounded-full transition-colors ' +
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
        </div>
        <span className="ml-2 hidden flex-none rounded-full bg-accent-lime/10 px-2 py-0.5 font-mono text-[10px] text-accent-lime ring-1 ring-accent-lime/20 sm:inline-block">
          ● connected · genai
        </span>
      </div>

      {/* Fields */}
      <div className="p-4 sm:p-6">
        <AnimatePresence mode="wait">
          <motion.div
            key={cur.id}
            initial={{ opacity: 0, y: 8, filter: 'blur(4px)' }}
            animate={{ opacity: 1, y: 0, filter: 'blur(0px)' }}
            exit={{ opacity: 0, y: -6, filter: 'blur(2px)' }}
            transition={{ duration: 0.35, ease: [0.22, 1, 0.36, 1] }}
            className="grid gap-4 sm:grid-cols-2"
          >
            {fields.map((f) => (
              <div key={f.k} className={f.full ? 'sm:col-span-2' : ''}>
                <p className="mb-1.5 text-[11px] font-medium uppercase tracking-wider text-zinc-500">{f.k}</p>
                <div
                  className={
                    'rounded-lg border border-white/5 bg-ink-950/60 px-3 py-2.5 text-sm text-zinc-200 ' +
                    (f.mono ? 'font-mono text-[12.5px]' : '')
                  }
                >
                  {f.v}
                </div>
              </div>
            ))}
          </motion.div>
        </AnimatePresence>
      </div>
    </div>
  );
}
