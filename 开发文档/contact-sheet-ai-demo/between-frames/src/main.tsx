import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import { MockRepository } from "./services/MockRepository";
import { desktopRuntime } from "./services/TauriRepository";
import { I18nProvider } from "./i18n";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <I18nProvider>
      <App repository={desktopRuntime() ? undefined : new MockRepository()} />
    </I18nProvider>
  </React.StrictMode>,
);
