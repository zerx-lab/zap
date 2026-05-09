import { defineConfig } from 'astro/config';
import mdx from '@astrojs/mdx';
import sitemap from '@astrojs/sitemap';

export default defineConfig({
  site: 'https://openwarp.dev',
  integrations: [mdx(), sitemap()],
  trailingSlash: 'ignore',
  redirects: {
    '/docs': '/docs/quickstart',
    '/docs/introduction': '/docs/quickstart',
    '/docs/install': '/docs/quickstart',
    '/docs/first-run': '/docs/quickstart',
  },
  build: {
    format: 'directory',
  },
});
