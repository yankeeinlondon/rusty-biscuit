import { resolve } from 'node:path'
import { defineConfig } from 'vite'
import VueRouter from 'unplugin-vue-router/vite'
import VueMacros from 'unplugin-vue-macros/vite'
import Vue from '@vitejs/plugin-vue'
import AutoImport from 'unplugin-auto-import/vite'
import Components from 'unplugin-vue-components/vite'
import Icons from 'unplugin-icons/vite'
import IconsResolver from 'unplugin-icons/resolver'
import Markdown from 'unplugin-vue-markdown/vite'
import UnoCSS from 'unocss/vite'
import VueDevTools from 'vite-plugin-vue-devtools'
import Inspect from 'vite-plugin-inspect'
import { VitePWA } from 'vite-plugin-pwa'
import type { HttpProxy } from 'vite'
import type { ServerResponse } from 'node:http'

/** Build proxy config that silently returns 502 when backend is down. */
function proxyConfig() {
  const target = 'http://localhost:3000'
  const paths = ['/status', '/health', '/sony_receiver', '/arcam_amp', '/arcam', '/eversolo', '/explore']

  function onProxyError(_err: Error, _req: unknown, res: ServerResponse) {
    res.writeHead(502, { 'Content-Type': 'application/json' })
    res.end(JSON.stringify({ error: 'Backend not running' }))
  }

  return Object.fromEntries(
    paths.map(path => [path, {
      target,
      configure: (proxy: HttpProxy.Server) => { proxy.on('error', onProxyError) },
    }]),
  )
}

export default defineConfig({
  resolve: {
    alias: {
      '~/': `${resolve(__dirname, 'src')}/`,
    },
  },

  plugins: [
    // File-based routing — must be before Vue
    VueRouter({
      dts: 'src/typed-router.d.ts',
    }),

    // Vue Macros wrapping Vue plugin
    VueMacros({
      plugins: {
        vue: Vue({
          include: [/\.vue$/, /\.md$/],
        }),
      },
    }),

    // Auto-import Vue/VueRouter/VueUse/Pinia APIs
    AutoImport({
      imports: [
        'vue',
        'vue-router',
        '@vueuse/core',
        'pinia',
      ],
      dirs: [
        'src/composables',
      ],
      dts: 'src/auto-imports.d.ts',
      vueTemplate: true,
    }),

    // Auto-register components + icon resolver
    Components({
      resolvers: [
        IconsResolver({
          prefix: 'i',
        }),
      ],
      dts: 'src/components.d.ts',
    }),

    // Icons (iconify)
    Icons({
      autoInstall: true,
    }),

    // Markdown as Vue components
    Markdown(),

    // UnoCSS
    UnoCSS(),

    // Devtools
    VueDevTools(),

    // Inspect
    Inspect(),

    // PWA
    VitePWA({
      registerType: 'autoUpdate',
      manifest: {
        name: 'Homelab Dashboard',
        short_name: 'Homelab',
        theme_color: '#1a1a2e',
      },
    }),
  ],

  server: {
    proxy: proxyConfig(),
  },

  test: {
    environment: 'jsdom',
    include: ['src/**/*.test.ts'],
  },
})
