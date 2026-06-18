---
prompt: |-
    The Reaper package area will want to be able to identify when a web page is using certain libraries, including:

    - frontend framework (vuejs, react, svelete, solid, etc.)
    - charting libraries (chartjs, plotly, vega, etc.)
    - animation frameworks
    - auth frameworks
    - ecommerce frameworks
    - CMS frameworks
    - CSS frameworks (UnoCSS, Tailwind, etc.)

    Your task is to act as an Orchestrator and:

    1. Find the top 10 frontend frameworks, and the top 5 charting, animation, auth, ecommerce, CMS, and CSS frameworks.
    2. For each framework create a subagent and have them provide you the following details on their framework focus:
        - provide a short description of the framework
        - provide key URLs (repo, docs, website)
        - what "signatures" might be detectable in a webpage which would indicate that this framework is in use
            - for each signature indicate the certainty with which this signature indicates that that framework is being used
        - would this signature be available with a simple HTTP request or would you need to fully hydrate the page with a crate like chromiumoxide?
    3. Once you have gathered and written the findings for each framework, add a "## Detection Approach" section which:
        - outlines 2-3 approachs that Reaper might take in detecting these frameworks
        - add pros/cons to each
last_updated: 2026-06-03
---
## Scope

Framework sets selected for Reaper detection:

- Frontend frameworks: React, Next.js, Angular, Vue.js, Svelte/SvelteKit, Nuxt, Astro, Remix/React Router, SolidJS/SolidStart, Qwik/Qwik City.
- Charting libraries: Chart.js, D3.js, Highcharts, Plotly.js, Apache ECharts.
- Animation frameworks: GSAP, Framer Motion/Motion, Anime.js, Lottie-web, AOS.
- Auth frameworks/platforms: Auth.js/NextAuth.js, Auth0, Clerk, Firebase Authentication, Supabase Auth.
- Ecommerce frameworks/platforms: Shopify, WooCommerce, Magento/Adobe Commerce, Wix Stores, Squarespace Commerce.
- CMS frameworks/platforms: WordPress, Drupal, Joomla, Webflow, Contentful.
- CSS frameworks: Tailwind CSS, Bootstrap, Material UI, Bulma, UnoCSS.

## Frontend Frameworks

### React

React is a JavaScript UI library for rendering component trees into the DOM. It is commonly used directly or through meta-frameworks such as Next.js, Remix/React Router, Gatsby, and others.

URLs:

- Website/docs: https://react.dev/
- Repository: https://github.com/facebook/react
- npm: https://www.npmjs.com/package/react

Signatures:

| Signature                                                                                                                                                            | Confidence                           | Availability                     |
|----------------------------------------------------------------------------------------------------------------------------------------------------------------------|--------------------------------------|----------------------------------|
| CDN or asset references to `react`, `react-dom`, `react.production.min.js`, `react-dom.production.min.js`, `unpkg.com/react`, `cdnjs.cloudflare.com/ajax/libs/react` | High when explicit                   | HTTP HTML/assets                 |
| Source maps, package manifests, exposed build metadata, or bundle text listing `react` and `react-dom`                                                               | High                                 | HTTP assets                      |
| `createRoot(`, `hydrateRoot(`, or imports from `react-dom/client` in visible JS                                                                                      | Medium-High                          | HTTP assets if source is visible |
| Legacy SSR attributes such as `data-reactroot`, `data-reactid`, `data-react-checksum`                                                                                | High for old React, low coverage now | HTTP HTML                        |
| DOM expando keys beginning `__reactFiber$`, `__reactProps$`, `__reactContainer$`, `__reactEvents$`                                                                   | High                                 | Hydrated browser                 |
| React DevTools hook with registered renderer: `window.__REACT_DEVTOOLS_GLOBAL_HOOK__` plus populated renderers                                                       | Medium-High                          | Hydrated browser                 |
| React Suspense/streaming comment markers such as `<!--$-->`, `<!--/$-->`, `<!--$?-->`                                                                                | Medium                               | HTTP HTML                        |

### Next.js

Next.js is a React framework for SSR, SSG, routing, server components, API routes, image optimization, and client hydration.

URLs:

- Website/docs: https://nextjs.org/
- Repository: https://github.com/vercel/next.js
- Docs: https://nextjs.org/docs

Signatures:

| Signature                                                                              | Confidence  | Availability        |
|----------------------------------------------------------------------------------------|-------------|---------------------|
| Asset paths under `/_next/static/`                                                     | High        | HTTP HTML/assets    |
| `<script id="__NEXT_DATA__" type="application/json">`                                  | High        | HTTP HTML           |
| Inline App Router payloads such as `self.__next_f.push(...)`                           | High        | HTTP HTML           |
| `/_next/data/{buildId}/...json` routes                                                 | High        | HTTP request/assets |
| Root container `<div id="__next">`                                                     | Medium-High | HTTP HTML           |
| Image optimizer URLs like `/_next/image?url=...&w=...&q=...` or `data-nimg` attributes | Medium-High | HTTP HTML/assets    |
| Bundle strings such as `next/dist/`, `next/router`, `next/navigation`, `__NEXT_DATA__` | Medium      | HTTP assets         |
| Headers such as `x-nextjs-cache`, `x-nextjs-stale-time`, `x-matched-path`              | Medium      | HTTP headers        |

### Angular

Angular is Google’s TypeScript-based web application platform for client-rendered, server-rendered, and hybrid web apps.

URLs:

- Website/docs: https://angular.dev/
- Repository: https://github.com/angular/angular
- CLI docs: https://angular.dev/tools/cli

Signatures:

| Signature                                                                                 | Confidence                 | Availability                                           |
|-------------------------------------------------------------------------------------------|----------------------------|--------------------------------------------------------|
| `ng-version="x.y.z"` on root/component elements                                           | High                       | Hydrated browser; HTTP HTML for SSR/SSG                |
| `_nghost-*` and `_ngcontent-*` attributes or CSS selectors                                | High                       | HTTP HTML/CSS for SSR/assets; hydrated browser for CSR |
| SSR/hydration markers such as `ngh`, `ng-server-context="ssr"`, `ng-server-context="ssg"` | High when present          | HTTP HTML                                              |
| `ngSkipHydration`                                                                         | High for Angular hydration | HTTP HTML or hydrated browser                          |
| Bundle strings such as `@angular/core`, `ɵɵdefineComponent`, `ɵcmp`, `ɵfac`, `ɵmod`       | Medium-High                | HTTP assets                                            |
| `3rdpartylicenses.txt` mentioning `@angular/*` packages                                   | Medium-High                | HTTP assets                                            |
| Runtime debug globals such as `window.ng.getComponent`                                    | High but dev-only          | Hydrated browser                                       |
| Angular CLI-like assets: `main.*.js`, `polyfills.*.js`, `runtime.*.js`                    | Low-Medium                 | HTTP HTML/assets                                       |

### Vue.js

Vue is a progressive JavaScript framework for building user interfaces, used through CDN/global builds, bundled SPAs, SSR, SSG, and Vue meta-frameworks.

URLs:

- Website/docs: https://vuejs.org/
- Repository: https://github.com/vuejs/core
- SSR docs: https://vuejs.org/guide/scaling-up/ssr

Signatures:

| Signature                                                                                                       | Confidence                 | Availability                                                |
|-----------------------------------------------------------------------------------------------------------------|----------------------------|-------------------------------------------------------------|
| CDN/import references to `vue.global.js`, `vue.esm-browser.js`, `unpkg.com/vue`, `jsdelivr.net/npm/vue`         | High                       | HTTP HTML                                                   |
| Compile flags such as `__VUE_OPTIONS_API__`, `__VUE_PROD_DEVTOOLS__`, `__VUE_PROD_HYDRATION_MISMATCH_DETAILS__` | High                       | HTTP assets                                                 |
| Root mount attribute `data-v-app`                                                                               | High                       | Hydrated browser; HTTP HTML only if prerendered after mount |
| DOM property `__vue_app__` on mount container                                                                   | High                       | Hydrated browser                                            |
| Scoped SFC selectors/attributes such as `data-v-xxxx` and `[data-v-xxxx]`                                       | Medium-High                | HTTP HTML/CSS or hydrated browser                           |
| Vue 2 SSR marker `data-server-rendered="true"`                                                                  | High for Vue 2 SSR         | HTTP HTML                                                   |
| Global `window.Vue`                                                                                             | High for CDN/global builds | Hydrated browser                                            |

### Svelte / SvelteKit

Svelte is a compiler-driven UI framework. SvelteKit is the official Svelte app framework for routing, SSR, SSG, adapters, and server features.

URLs:

- Website/docs: https://svelte.dev/
- Svelte repository: https://github.com/sveltejs/svelte
- SvelteKit repository: https://github.com/sveltejs/kit

Signatures:

| Signature                                                                                                                        | Confidence      | Availability             |
|----------------------------------------------------------------------------------------------------------------------------------|-----------------|--------------------------|
| CSS classes/selectors matching `svelte-[a-z0-9]+`                                                                                | High            | HTTP HTML/CSS/assets     |
| SvelteKit asset paths such as `/_app/immutable/entry/start.*.js`, `/_app/immutable/entry/app.*.js`, `/_app/immutable/nodes/*.js` | Very High       | HTTP HTML/assets         |
| Inline bootstrap global such as `__sveltekit_<id>`                                                                               | Very High       | HTTP HTML                |
| `data-sveltekit-preload-data`, `data-sveltekit-preload-code`, `data-sveltekit-reload`, `data-sveltekit-noscroll`                 | Medium-High     | HTTP HTML                |
| Bundle/source-map strings such as `svelte/internal`, `svelte/internal/client`, `$app/navigation`, `@sveltejs/kit`                | Medium          | HTTP assets              |
| Source template placeholders such as `%sveltekit.head%`, `%sveltekit.body%`                                                      | High if exposed | HTTP assets/source leaks |

### Nuxt

Nuxt is a Vue meta-framework for SSR, SSG, SPA, hybrid rendering, routing, data fetching, and full-stack server features through Nitro.

URLs:

- Website/docs: https://nuxt.com/
- Repository: https://github.com/nuxt/nuxt
- Docs: https://nuxt.com/docs

Signatures:

| Signature                                                                  | Confidence | Availability        |
|----------------------------------------------------------------------------|------------|---------------------|
| `<script id="__NUXT_DATA__" type="application/json" data-nuxt-data="...">` | Very High  | HTTP HTML           |
| `data-nuxt-data="nuxt-app"` and `data-ssr="true"` or `data-ssr="false"`    | Very High  | HTTP HTML           |
| Inline `window.__NUXT__` or `window.__NUXT__.config`                       | High       | HTTP HTML           |
| Asset paths under `/_nuxt/`                                                | Medium     | HTTP HTML/assets    |
| Root element `<div id="__nuxt">`                                           | Medium     | HTTP HTML           |
| Payload routes such as `/_payload.json`                                    | High       | HTTP request/assets |
| Runtime globals such as `window.useNuxtApp` or Vue `$nuxt` property        | High       | Hydrated browser    |

### Astro

Astro is a JavaScript web framework for content-driven sites that renders HTML by default and hydrates optional client/server islands.

URLs:

- Website: https://astro.build/
- Docs: https://docs.astro.build/
- Repository: https://github.com/withastro/astro

Signatures:

| Signature                                                                                                   | Confidence                | Availability             |
|-------------------------------------------------------------------------------------------------------------|---------------------------|--------------------------|
| `<astro-island ...>` elements                                                                               | High                      | HTTP HTML                |
| Island attributes such as `component-url`, `renderer-url`, `component-export`, `props`, `client`, `ssr`     | High                      | HTTP HTML                |
| Runtime strings such as `customElements.define("astro-island"`                                              | High                      | HTTP HTML/assets         |
| Slot markers such as `astro-slot`, `astro-static-slot`, `template[data-astro-template]`, `<!--astro:end-->` | High with island evidence | HTTP HTML                |
| Server island URLs under `/_server-islands/`                                                                | High                      | HTTP HTML/assets/network |
| Scoped CSS attributes/selectors such as `data-astro-cid-*`                                                  | Medium                    | HTTP HTML/CSS            |
| Asset paths under `/_astro/`                                                                                | Low-Medium                | HTTP HTML/assets         |
| `<meta name="generator" content="Astro ...">`                                                               | Medium                    | HTTP HTML                |

### Remix / React Router

Remix is a full-stack React framework whose current framework path has largely converged into React Router framework mode.

URLs:

- React Router website/docs: https://reactrouter.com/
- React Router repository: https://github.com/remix-run/react-router
- Remix website: https://remix.run/
- Remix repository: https://github.com/remix-run/remix

Signatures:

| Signature                                                                                             | Confidence  | Availability      |
|-------------------------------------------------------------------------------------------------------|-------------|-------------------|
| `window.__reactRouterContext`                                                                         | Very High   | HTTP HTML         |
| `window.__reactRouterManifest`                                                                        | Very High   | HTTP HTML         |
| `window.__reactRouterRouteModules`                                                                    | Very High   | HTTP HTML         |
| `/__manifest` endpoint or manifest path in route discovery data                                       | High        | HTTP request/HTML |
| `data-discover="true"` on links                                                                       | Medium-High | HTTP HTML         |
| `react-router-scroll-positions`                                                                       | Medium-High | HTTP HTML         |
| Legacy Remix globals: `window.__remixContext`, `window.__remixManifest`, `window.__remixRouteModules` | High        | HTTP HTML         |
| `data-remix-managed-head`                                                                             | Medium      | HTTP HTML         |
| Vite-like assets such as `/assets/entry.client-*.js`, `/assets/root-*.js`                             | Medium      | HTTP assets       |

### SolidJS / SolidStart

SolidJS is a fine-grained reactive UI library. SolidStart is the Solid meta-framework for SSR, SSG, routing, server functions, and deployment presets.

URLs:

- Solid website: https://www.solidjs.com/
- Docs: https://docs.solidjs.com/
- Solid repository: https://github.com/solidjs/solid
- SolidStart repository: https://github.com/solidjs/solid-start

Signatures:

| Signature                                                                               | Confidence        | Availability                        |
|-----------------------------------------------------------------------------------------|-------------------|-------------------------------------|
| `window._$HY` hydration global/script                                                   | High              | HTTP HTML for SSR; hydrated browser |
| `data-hk="..."` hydration markers                                                       | High              | HTTP HTML for SSR                   |
| Solid SSR/Suspense comments such as `<!--!$...-->` and `<!--!$/...-->`                  | Medium            | HTTP HTML                           |
| Bundle/source-map strings such as `solid-js`, `solid-js/web`, `@solidjs/router`         | High when visible | HTTP assets                         |
| SolidStart strings such as `@solidjs/start`, `StartClient`, `StartServer`, `FileRoutes` | High when visible | HTTP assets/source maps             |
| Vinxi/Nitro/Vite-only clues                                                             | Low               | HTTP assets                         |

### Qwik / Qwik City

Qwik is a resumable frontend framework that serializes state and event handlers into HTML. Qwik City adds routing, layouts, endpoints, SSR, and SSG.

URLs:

- Website/docs: https://qwik.dev/
- Repository: https://github.com/QwikDev/qwik
- Qwik City docs: https://qwik.dev/docs/qwikcity/

Signatures:

| Signature                                                                          | Confidence         | Availability |
|------------------------------------------------------------------------------------|--------------------|--------------|
| Root/container attributes such as `q:container`, `q:version`, `q:render`, `q:base` | Very High          | HTTP HTML    |
| `q:manifest-hash`, `q:instance`                                                    | High               | HTTP HTML    |
| `<script type="qwik/json">` or `q:func="qwik/json"`                                | Very High          | HTTP HTML    |
| Serialized event QRLs such as `on:click="./chunk.js#Symbol[0,1]"`                  | Very High          | HTTP HTML    |
| QRL patterns containing `.js#SymbolName` and lexical indexes                       | High               | HTTP HTML    |
| `q:route="/path"`                                                                  | High for Qwik City | HTTP HTML    |
| `q-manifest.json`, `q-bundle-graph-*.json`, `/build/q-*.js`                        | Medium             | HTTP assets  |
| `@builder.io/qwik-city` in source maps or unminified bundles                       | High when visible  | HTTP assets  |

## Charting Libraries

| Library        | Description                                                                                 | URLs                                                                                      | Signatures                                                                                                                                                                                                                                               |
|----------------|---------------------------------------------------------------------------------------------|-------------------------------------------------------------------------------------------|----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| Chart.js       | Canvas-based charting library for common chart types.                                       | Website/docs: https://www.chartjs.org/; repo: https://github.com/chartjs/Chart.js         | High: `cdn.jsdelivr.net/npm/chart.js`, `cdnjs.cloudflare.com/ajax/libs/Chart.js`, `Chart.js v`, `window.Chart`, `Chart.version`. Medium-High: `new Chart(ctx, ...)`, `chartjs-plugin-*`. HTTP for script/assets; hydrated browser for globals/instances. |
| D3.js          | Low-level data visualization library for binding data to SVG/HTML/Canvas.                   | Website/docs: https://d3js.org/; repo: https://github.com/d3/d3                           | High: CDN/package names `d3`, `d3.min.js`, `d3.v7.min.js`, `d3-selection`, `d3-scale`, `window.d3`. Medium: code strings like `d3.select`, `d3.scaleLinear`, `d3.axisBottom`. HTTP for script/assets; hydrated browser for `window.d3`.                  |
| Highcharts     | Commercial JavaScript charting library for interactive SVG charts.                          | Website/docs: https://www.highcharts.com/; repo: https://github.com/highcharts/highcharts | High: `code.highcharts.com/highcharts.js`, `Highcharts.chart`, `Highcharts Stock`, `window.Highcharts`, `Highcharts.version`. Medium: SVG credits text `Highcharts.com`. HTTP for scripts/assets/HTML; hydrated browser for globals/rendered charts.     |
| Plotly.js      | Declarative charting library for scientific, statistical, 3D, and dashboard visualizations. | Website/docs: https://plotly.com/javascript/; repo: https://github.com/plotly/plotly.js   | High: `cdn.plot.ly/plotly`, `plotly.js`, `window.Plotly`, `Plotly.newPlot`, `data-plotly`. Medium: `.plotly-graph-div`. HTTP for scripts/assets/HTML; hydrated browser for `window.Plotly` and rendered graph divs.                                      |
| Apache ECharts | Canvas/SVG charting and visualization library from Apache.                                  | Website/docs: https://echarts.apache.org/; repo: https://github.com/apache/echarts        | High: `echarts.min.js`, `cdn.jsdelivr.net/npm/echarts`, `window.echarts`, `echarts.init`. Medium: DOM attributes/classes such as `_echarts_instance_`. HTTP for scripts/assets; hydrated browser for globals/instances.                                  |

## Animation Frameworks

| Framework              | Description                                                                                      | URLs                                                                                     | Signatures                                                                                                                                                                                                                                                 |
|------------------------|--------------------------------------------------------------------------------------------------|------------------------------------------------------------------------------------------|------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| GSAP                   | Professional JavaScript animation platform for timelines, tweens, scroll animation, and plugins. | Website/docs: https://gsap.com/; repo: https://github.com/greensock/GSAP                 | High: `gsap.min.js`, `cdn.jsdelivr.net/npm/gsap`, `window.gsap`, `gsap.to`, `gsap.timeline`, plugin names like `ScrollTrigger`. HTTP for scripts/assets/source; hydrated browser for globals.                                                              |
| Framer Motion / Motion | React-first animation library now under the Motion ecosystem.                                    | Website/docs: https://motion.dev/; repo: https://github.com/motiondivision/motion        | High: package/source strings `framer-motion`, `motion/react`, `motion.dev`. Medium: React component strings like `AnimatePresence`, `motion.div`. HTTP if source maps/assets expose names; hydrated browser is usually needed for behavioral confirmation. |
| Anime.js               | Lightweight JavaScript animation engine for CSS properties, SVG, DOM attributes, and JS objects. | Website/docs: https://animejs.com/; repo: https://github.com/juliangarnier/anime         | High: `anime.min.js`, `window.anime`, `anime({ ... })`. Medium: CDN paths containing `animejs`. HTTP for scripts/assets/source; hydrated browser for global runtime.                                                                                       |
| Lottie-web             | Renderer for Bodymovin/Lottie JSON animations using SVG, Canvas, or HTML.                        | Website/docs: https://airbnb.io/lottie/; repo: https://github.com/airbnb/lottie-web      | High: `lottie.min.js`, `bodymovin`, `window.lottie`, `lottie.loadAnimation`, `.json` animation assets with `v`, `fr`, `ip`, `op`, `layers`. HTTP for scripts/assets/JSON; hydrated browser for globals/rendered SVG.                                       |
| AOS                    | Animate On Scroll library for scroll-triggered CSS animations.                                   | Website/docs: https://michalsnik.github.io/aos/; repo: https://github.com/michalsnik/aos | High: `aos.css`, `aos.js`, `data-aos`, `AOS.init`. HTTP for HTML/scripts/CSS; hydrated browser for `window.AOS`.                                                                                                                                           |

## Auth Frameworks And Platforms

| Framework               | Description                                                                 | URLs                                                                                                       | Signatures                                                                                                                                                                                                                                        |
|-------------------------|-----------------------------------------------------------------------------|------------------------------------------------------------------------------------------------------------|---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| Auth.js / NextAuth.js   | Open-source authentication framework for Next.js and modern web apps.       | Website/docs: https://authjs.dev/; repo: https://github.com/nextauthjs/next-auth                           | High: routes such as `/api/auth/session`, `/api/auth/providers`, `/api/auth/csrf`, cookies `next-auth.session-token`, `__Secure-next-auth.session-token`, `authjs.session-token`. HTTP for route probing/cookies; hydrated browser rarely needed. |
| Auth0                   | Hosted identity platform using OAuth/OIDC, Universal Login, and SDKs.       | Website/docs: https://auth0.com/docs; SDK repo: https://github.com/auth0/auth0-spa-js                      | High: domains `*.auth0.com`, `auth0.com/authorize`, `auth0-spa-js`, `auth0Client`, cookies or redirects involving Auth0 tenant domains. HTTP for HTML/assets/redirects; hydrated browser for SDK globals/token flows.                             |
| Clerk                   | Hosted authentication and user management for React, Next.js, and web apps. | Website/docs: https://clerk.com/docs; repo: https://github.com/clerk/javascript                            | High: `clerk.com`, `clerk-js`, `@clerk/clerk-js`, `__clerk_db_jwt`, `Clerk.load`, `window.Clerk`. HTTP for scripts/cookies; hydrated browser for `window.Clerk`.                                                                                  |
| Firebase Authentication | Google Firebase identity service for web and mobile apps.                   | Website/docs: https://firebase.google.com/docs/auth; SDK repo: https://github.com/firebase/firebase-js-sdk | High: `firebase-auth.js`, `firebaseapp.com`, `firebaseConfig`, `apiKey`, `authDomain`, imports from `firebase/auth`. Medium: Identity Toolkit endpoints. HTTP for HTML/assets/config; hydrated browser for initialized Firebase app/auth state.   |
| Supabase Auth           | Auth service built around Supabase projects and GoTrue.                     | Website/docs: https://supabase.com/docs/guides/auth; JS repo: https://github.com/supabase/supabase-js      | High: `@supabase/supabase-js`, `createClient(` with `supabase.co`, `/auth/v1/`, cookies/storage keys containing `sb-`. HTTP for assets/config/endpoints; hydrated browser for localStorage/session inspection.                                    |

## Ecommerce Frameworks And Platforms

| Framework                | Description                                                                             | URLs                                                                                                   | Signatures                                                                                                                                                                                                         |
|--------------------------|-----------------------------------------------------------------------------------------|--------------------------------------------------------------------------------------------------------|--------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| Shopify                  | Hosted ecommerce platform with storefronts, checkout, cart, product, and app ecosystem. | Website/docs: https://shopify.dev/; main: https://www.shopify.com/                                     | High: `cdn.shopify.com`, `Shopify.theme`, `window.Shopify`, `/cart.js`, `/products/*.js`, `/checkout`, cookies `_shopify_*`, `cart_sig`. HTTP for HTML/assets/routes/cookies; hydrated browser for globals.        |
| WooCommerce              | WordPress ecommerce plugin for products, carts, checkout, and payments.                 | Website/docs: https://woocommerce.com/documentation/; repo: https://github.com/woocommerce/woocommerce | High: `wp-content/plugins/woocommerce`, `woocommerce`, `wc-cart-fragments`, `wc-ajax`, `woocommerce_items_in_cart`, `/cart/`, `/checkout/`. HTTP for HTML/assets/cookies/routes; hydrated browser rarely needed.   |
| Magento / Adobe Commerce | PHP ecommerce platform for catalogs, carts, checkout, and enterprise commerce.          | Website/docs: https://developer.adobe.com/commerce/; repo: https://github.com/magento/magento2         | High: `/static/frontend/`, `/pub/static/`, `Magento_`, `mage/`, `requirejs-config.js`, cookies `mage-cache-storage`, `form_key`. HTTP for HTML/assets/cookies; hydrated browser for RequireJS/module state.        |
| Wix Stores               | Hosted Wix ecommerce storefront feature.                                                | Website/docs: https://dev.wix.com/; main: https://www.wix.com/ecommerce                                | High: `wixstatic.com`, `static.parastorage.com`, `wixstores`, `WixStores`, `X-Wix-*` headers. Medium: `window.wixBiSession`. HTTP for HTML/assets/headers; hydrated browser for globals.                           |
| Squarespace Commerce     | Squarespace ecommerce functionality for hosted websites.                                | Website/docs: https://developers.squarespace.com/; main: https://www.squarespace.com/ecommerce         | High: `static1.squarespace.com`, `squarespace.com`, `Y.Squarespace`, `squarespace-commerce`, `/commerce/`, cart/product JSON endpoints. HTTP for HTML/assets/routes; hydrated browser for `Y.Squarespace` runtime. |

## CMS Frameworks And Platforms

| CMS        | Description                                                               | URLs                                                                                                                | Signatures                                                                                                                                                                                                                                                      |
|------------|---------------------------------------------------------------------------|---------------------------------------------------------------------------------------------------------------------|-----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| WordPress  | Dominant PHP CMS with themes, plugins, REST API, and block editor.        | Website/docs: https://wordpress.org/documentation/; repo: https://github.com/WordPress/wordpress-develop            | High: `/wp-content/`, `/wp-includes/`, `/wp-json/`, `<meta name="generator" content="WordPress ...">`. Medium: cookies `wordpress_logged_in_*`, `wp-settings-*`. HTTP for HTML/assets/routes/cookies.                                                           |
| Drupal     | PHP CMS/framework for structured content and enterprise websites.         | Website/docs: https://www.drupal.org/docs; repo: https://git.drupalcode.org/project/drupal                          | High: `/sites/default/`, `drupalSettings`, `Drupal.behaviors`, `<meta name="Generator" content="Drupal ...">`. Medium: cookies beginning `SSESS`/`SESS`. HTTP for HTML/assets/cookies; hydrated browser for `drupalSettings`.                                   |
| Joomla     | PHP CMS for websites, templates, extensions, and content management.      | Website/docs: https://docs.joomla.org/; repo: https://github.com/joomla/joomla-cms                                  | High: `/media/system/`, `/media/jui/`, `Joomla!`, `<meta name="generator" content="Joomla!">`. Medium: cookies or routes with `com_content`, `com_users`. HTTP for HTML/assets/routes.                                                                          |
| Webflow    | Hosted visual website builder/CMS with generated static frontend assets.  | Website/docs: https://developers.webflow.com/; main: https://webflow.com/                                           | High: `webflow.js`, `webflow.css`, `data-wf-page`, `data-wf-site`, `w-dyn-list`, `w-dyn-item`. HTTP for HTML/assets; hydrated browser for `Webflow` runtime.                                                                                                    |
| Contentful | Headless CMS used through APIs and SDKs rather than a fixed page runtime. | Website/docs: https://www.contentful.com/developers/docs/; JS SDK repo: https://github.com/contentful/contentful.js | High: `cdn.contentful.com`, `preview.contentful.com`, `contentful.js`, `createClient({ space, accessToken })`. Medium: GraphQL endpoint `graphql.contentful.com/content/v1/spaces`. HTTP for assets/config/network URLs; hydrated browser for SDK/client state. |

## CSS Frameworks

| Framework    | Description                                                          | URLs                                                                                      | Signatures                                                                                                                                                                                                                                                    |
|--------------|----------------------------------------------------------------------|-------------------------------------------------------------------------------------------|---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| Tailwind CSS | Utility-first CSS framework compiled into project CSS.               | Website/docs: https://tailwindcss.com/; repo: https://github.com/tailwindlabs/tailwindcss | Medium-High: dense utility classes such as `flex`, `grid`, `text-*`, `bg-*`, `sm:`, `md:`, `dark:`, arbitrary values like `w-[...]`. High if source maps/assets expose `tailwindcss` or config. HTTP HTML/CSS/assets.                                         |
| Bootstrap    | CSS and JS component framework with grid, utilities, and components. | Website/docs: https://getbootstrap.com/; repo: https://github.com/twbs/bootstrap          | High: `bootstrap.min.css`, `bootstrap.bundle.min.js`, `cdn.jsdelivr.net/npm/bootstrap`, `window.bootstrap`. Medium: classes `container`, `row`, `col-*`, `navbar`, `btn`, `modal`. HTTP for HTML/assets; hydrated browser for JS globals.                     |
| Material UI  | React component library implementing Material Design.                | Website/docs: https://mui.com/; repo: https://github.com/mui/material-ui                  | High: package/source strings `@mui/material`, `MuiButton`, `MuiTypography`, Emotion class prefixes in source maps. Medium: runtime classes like `MuiButton-root`, `MuiFormControl-root`. HTTP HTML/assets; hydrated browser for client-rendered class output. |
| Bulma        | CSS-only framework with utility and component classes.               | Website/docs: https://bulma.io/; repo: https://github.com/jgthms/bulma                    | High: `bulma.min.css`, CDN/package paths. Medium: class combinations such as `is-primary`, `is-danger`, `columns`, `column`, `navbar`, `hero`, `section`. HTTP HTML/CSS/assets.                                                                               |
| UnoCSS       | Atomic CSS engine that generates on-demand utilities and presets.    | Website/docs: https://unocss.dev/; repo: https://github.com/unocss/unocss                 | High: assets/source maps exposing `unocss`, `@unocss`, `uno.css`, `__uno.css`. Medium: atomic utilities with Uno-specific variants or attributify syntax such as `text="..."`, `bg="..."`, `i-...`. HTTP HTML/CSS/assets; hydrated browser rarely needed.     |

## Detection Approach

### Approach 1: Passive HTTP Fingerprinting

Fetch the page HTML, headers, cookies, linked scripts, linked stylesheets, source maps when publicly available, and a small bounded set of obvious framework routes.

Pros:

- Fast and cheap.
- Works well for SSR, SSG, hosted platforms, CDNs, exposed package names, cookies, headers, and static asset paths.
- Avoids browser startup cost and JavaScript execution risk.
- Good first pass for high-confidence signatures such as `/_next/static/`, `__NUXT_DATA__`, `<astro-island>`, `q:container`, `/wp-content/`, `cdn.shopify.com`.

Cons:

- Weak for CSR-only applications whose initial HTML is mostly empty.
- Bundlers and minifiers can erase package names.
- CSS utility frameworks can be hard to distinguish from hand-authored classes.
- Some signatures are configurable and should not be used alone.

### Approach 2: Asset Graph And Weighted Signature Scoring

Build a detector registry where each framework has weighted signatures across HTML, headers, cookies, routes, script URLs, CSS URLs, fetched JS/CSS contents, source maps, and optional route probes. Sum evidence into confidence levels instead of returning a boolean from one match.

Pros:

- Handles overlapping ecosystems, such as React plus Next.js, Vue plus Nuxt, Svelte plus SvelteKit, and WordPress plus WooCommerce.
- Reduces false positives from generic markers like `canvas`, `container`, `row`, or Vite-style chunks.
- Produces explainable results: Reaper can report exactly which signatures were found.
- Lets Reaper prefer specific frameworks over base libraries when both are detected.

Cons:

- Requires curated signatures and ongoing maintenance.
- Needs careful normalization to avoid over-counting the same evidence from repeated assets.
- More network requests than HTML-only detection.
- Source-map fetching must be bounded and respectful.

### Approach 3: Hydrated Browser Confirmation

Use a headless browser, such as a chromiumoxide-backed path, after passive detection is inconclusive or when higher confidence is needed. Inspect post-hydration DOM, globals, localStorage/sessionStorage, network requests, dynamically imported chunks, service workers, and runtime object properties.

Pros:

- Finds runtime-only evidence such as `window.Chart`, `window.Highcharts`, `window.Clerk`, `window._$HY`, React fiber properties, Vue `__vue_app__`, and client-rendered CSS/component classes.
- Better for CSR-only SPAs.
- Can observe auth/ecommerce SDK initialization and dynamic chart/animation libraries.
- Can distinguish libraries that only appear after route changes or lazy-loaded interactions.

Cons:

- Slow and resource-intensive.
- Increases operational complexity: browser lifecycle, timeouts, bot defenses, CSP, redirects, consent banners, and flaky dynamic behavior.
- May trigger analytics, auth redirects, or anti-abuse systems.
- Some evidence is extension/devtools-dependent and should be treated carefully.

A practical Reaper pipeline should start with passive HTTP fingerprinting, then run weighted asset scoring, and only hydrate pages when the result is ambiguous, high-value, or likely CSR-only.
