export const csr = true;
export const prerender = true;
export const ssr = false;
export const trailingSlash = 'always';

import { browser } from "$app/environment";
if (browser) {
    // 诊断：主题 link 是否真的加载成功。
    // 主题变量全在 external CSS 里，加载失败时 var(--mantle) 为空，
    // 所有依赖它的 background-color 静默失效 —— 表现为大片白。
    const probe = () => {
        const link = document.querySelector('link[href*="themes/"]');
        const cs = getComputedStyle(document.documentElement);
        const info = [
            "href=" + (link ? link.getAttribute("href") : "NONE"),
            "cssLoaded=" + (link && link.sheet ? "yes" : "no"),
            "mantle=" + (cs.getPropertyValue("--mantle").trim() || "EMPTY"),
        ].join(" | ");
        console.log("[theme-diag] " + info);
        document.documentElement.setAttribute("data-theme-diag", info);
    };
    setTimeout(probe, 2000);
}
