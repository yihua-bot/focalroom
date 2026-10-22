
import React from "react";import {createRoot} from "react-dom/client";import {QueryClient,QueryClientProvider} from "@tanstack/react-query";
import SharedApp from "../../../开发文档/contact-sheet-ai-demo/between-frames/apps/web/src/SharedApp.tsx";import {WebI18nProvider} from "../../../开发文档/contact-sheet-ai-demo/between-frames/apps/web/src/i18n.tsx";
localStorage.setItem("focalroom-locale","zh");createRoot(document.getElementById("root")!).render(<QueryClientProvider client={new QueryClient()}><WebI18nProvider><SharedApp/></WebI18nProvider></QueryClientProvider>);
