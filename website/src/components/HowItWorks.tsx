import { useEffect, useRef, useState } from "react";
import { motion, AnimatePresence } from "framer-motion";

type Step = { num: string; title: string; desc: string };
interface Props {
  steps: Step[];
}

export default function HowItWorks({ steps }: Props) {
  const [active, setActive] = useState(0);
  const refs = useRef<(HTMLDivElement | null)[]>([]);

  useEffect(() => {
    const observers = refs.current.map((el, i) => {
      if (!el) return null;
      const ob = new IntersectionObserver(
        ([entry]) => {
          if (entry.isIntersecting) setActive(i);
        },
        { rootMargin: "-45% 0px -45% 0px", threshold: 0 },
      );
      ob.observe(el);
      return ob;
    });
    return () => observers.forEach((o) => o?.disconnect());
  }, []);

  return (
    <div className="relative mt-14 grid grid-cols-1 gap-8 lg:grid-cols-12 lg:gap-14">
      <div className="space-y-4 lg:col-span-5">
        {steps.map((s, i) => (
          <div
            key={i}
            ref={(el) => {
              refs.current[i] = el;
            }}
            tabIndex={0}
            onMouseEnter={() => setActive(i)}
            onFocus={() => setActive(i)}
            className={
              "rounded-3xl border p-5 transition duration-500 " +
              (active === i
                ? "border-white/15 bg-white/[0.055] shadow-card"
                : "border-white/5 bg-white/[0.02] hover:border-white/10")
            }
          >
            <div
              className={
                "flex items-center gap-3 font-mono text-xs transition-colors duration-500 " +
                (active === i ? "text-brand-300" : "text-zinc-600")
              }
            >
              <span
                className={
                  "h-px w-8 transition-all duration-500 " +
                  (active === i ? "bg-brand-400" : "bg-zinc-700")
                }
              />
              {s.num}
            </div>
            <h3
              className={
                "mt-3 font-display text-2xl font-semibold tracking-tight transition-colors duration-500 sm:text-3xl " +
                (active === i ? "text-white" : "text-zinc-500")
              }
            >
              {s.title}
            </h3>
            <p
              className={
                "mt-3 max-w-md text-[15px] leading-relaxed transition-colors duration-500 " +
                (active === i ? "text-zinc-300" : "text-zinc-600")
              }
            >
              {s.desc}
            </p>
          </div>
        ))}
      </div>

      <div className="hidden lg:col-span-7 lg:block">
        <div className="sticky top-32">
          <div className="surface-card relative h-[520px] overflow-hidden">
            {/* 跟随当前步骤移动的径向高光 */}
            <div
              aria-hidden
              className="pointer-events-none absolute inset-0 transition-all duration-700"
              style={{
                background: `radial-gradient(35% 40% at ${20 + active * 30}% 30%, rgba(141,125,255,0.18), transparent 70%)`,
              }}
            />

            <div className="absolute right-4 top-4 z-10 flex items-center gap-1.5">
              {steps.map((_, i) => (
                <span
                  key={i}
                  className={
                    "h-1.5 rounded-full transition-all duration-500 " +
                    (active === i ? "w-6 bg-brand-400" : "w-1.5 bg-zinc-700")
                  }
                />
              ))}
            </div>

            <AnimatePresence mode="wait">
              {active === 0 && <StepConfigure key="0" />}
              {active === 1 && <StepTemplate key="1" />}
              {active === 2 && <StepUse key="2" />}
            </AnimatePresence>
          </div>
        </div>
      </div>
    </div>
  );
}

const fadeIn = {
  initial: { opacity: 0, y: 16 },
  animate: {
    opacity: 1,
    y: 0,
    transition: { duration: 0.5, ease: [0.22, 1, 0.36, 1] },
  },
  exit: { opacity: 0, y: -8, transition: { duration: 0.25 } },
};

function StepConfigure() {
  return (
    <motion.div
      {...fadeIn}
      className="absolute inset-0 flex items-center justify-center p-10"
    >
      <div className="w-full max-w-md rounded-xl border border-white/10 bg-ink-900/80 p-5 shadow-card">
        <div className="mb-4 flex items-center justify-between">
          <span className="text-[11px] font-medium uppercase tracking-widest text-zinc-500">
            Custom Provider
          </span>
          <span className="rounded-full bg-accent-lime/10 px-2 py-0.5 font-mono text-[10px] text-accent-lime ring-1 ring-accent-lime/20">
            connected
          </span>
        </div>
        {[
          ["name", "DeepSeek (Personal)"],
          ["base_url", "https://api.deepseek.com/v1"],
          ["api_key", "sk-••••••••••••••••••••"],
          ["model", "deepseek-r1"],
        ].map(([k, v], i) => (
          <motion.div
            key={k}
            initial={{ opacity: 0, x: -8 }}
            animate={{ opacity: 1, x: 0 }}
            transition={{ delay: 0.18 + i * 0.08 }}
            className="mt-3 flex items-center gap-3 border-t border-white/5 pt-3 first:border-0 first:pt-0"
          >
            <span className="w-20 flex-none font-mono text-[11px] text-zinc-500">
              {k}
            </span>
            <span className="truncate font-mono text-[12.5px] text-zinc-100">
              {v}
            </span>
          </motion.div>
        ))}
      </div>
    </motion.div>
  );
}

function StepTemplate() {
  const tpl = `{% if user.role %}
You are an expert {{ user.role }}.
Workspace: {{ cwd }}
{% endif %}
Reply concisely in {{ locale }}.`;
  const rendered = `You are an expert Rust engineer.
Workspace: ~/projects/openwarp
Reply concisely in zh-CN.`;
  return (
    <motion.div {...fadeIn} className="absolute inset-0 flex items-center p-8">
      <div className="grid w-full grid-cols-2 gap-4">
        <div className="rounded-xl border border-white/10 bg-ink-900/80 p-4">
          <div className="mb-2 font-mono text-[10px] uppercase tracking-widest text-zinc-500">
            template
          </div>
          <pre className="overflow-hidden font-mono text-[11.5px] leading-6 text-zinc-300 whitespace-pre-wrap">
            {tpl.split("\n").map((ln, i) => (
              <motion.div
                key={i}
                initial={{ opacity: 0, x: -4 }}
                animate={{ opacity: 1, x: 0 }}
                transition={{ delay: 0.1 + i * 0.06 }}
              >
                <span className="mr-3 select-none text-zinc-700">{i + 1}</span>
                <span
                  dangerouslySetInnerHTML={{
                    __html: ln
                      .replaceAll(
                        /(\{\%[^%]*\%\})/g,
                        '<span style="color:#ff9ec7">$1</span>',
                      )
                      .replaceAll(
                        /(\{\{[^}]*\}\})/g,
                        '<span style="color:#8be7ff">$1</span>',
                      ),
                  }}
                />
              </motion.div>
            ))}
          </pre>
        </div>
        <div className="rounded-xl border border-brand-500/20 bg-ink-900/80 p-4">
          <div className="mb-2 flex items-center gap-2 font-mono text-[10px] uppercase tracking-widest text-brand-300">
            <svg
              viewBox="0 0 24 24"
              className="h-3 w-3"
              fill="none"
              stroke="currentColor"
              strokeWidth="2.5"
            >
              <path d="M5 12h14M13 6l6 6-6 6" />
            </svg>
            rendered
          </div>
          <pre className="font-mono text-[11.5px] leading-6 text-zinc-200 whitespace-pre-wrap">
            {rendered.split("").map((c, i) => (
              <motion.span
                key={i}
                initial={{ opacity: 0 }}
                animate={{ opacity: 1 }}
                transition={{ delay: 0.6 + i * 0.008, duration: 0.2 }}
              >
                {c}
              </motion.span>
            ))}
          </pre>
        </div>
      </div>
    </motion.div>
  );
}

function StepUse() {
  return (
    <motion.div
      {...fadeIn}
      className="absolute inset-0 flex items-center justify-center p-10"
    >
      <div className="w-full max-w-lg rounded-xl border border-white/10 bg-ink-950/90 p-5 font-mono text-[12.5px]">
        <div className="mb-3 flex items-center gap-1.5">
          <span className="h-2.5 w-2.5 rounded-full bg-[#ff5f57]" />
          <span className="h-2.5 w-2.5 rounded-full bg-[#febc2e]" />
          <span className="h-2.5 w-2.5 rounded-full bg-[#28c840]" />
          <span className="ml-3 text-[10px] text-zinc-500">
            openwarp · ready
          </span>
        </div>
        {[
          { t: "$ openwarp", delay: 0 },
          { t: "✓ deepseek-r1 connected · 42ms", tone: "lime", delay: 0.4 },
          { t: "❯ /model qwen2.5-coder", delay: 0.85 },
          { t: "◆ switched · context preserved", tone: "cyan", delay: 1.2 },
          { t: '❯ "explain this stack trace"', delay: 1.65 },
          {
            t: "Looking at frame 3, the panic happens",
            tone: "soft",
            delay: 2.1,
          },
          {
            t: "because the buffer is dropped before…",
            tone: "soft",
            delay: 2.5,
          },
        ].map((ln, i) => (
          <motion.div
            key={i}
            initial={{ opacity: 0, x: -6 }}
            animate={{ opacity: 1, x: 0 }}
            transition={{ delay: ln.delay }}
            className={
              ln.tone === "lime"
                ? "text-accent-lime"
                : ln.tone === "cyan"
                  ? "text-accent-cyan"
                  : ln.tone === "soft"
                    ? "text-zinc-400 pl-3"
                    : "text-zinc-100"
            }
          >
            {ln.t}
          </motion.div>
        ))}
      </div>
    </motion.div>
  );
}
