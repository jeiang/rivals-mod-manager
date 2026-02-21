/* @refresh reload */
import { render } from "solid-js/web";
import "./index.css";
import { Route, Router } from "@solidjs/router";
import Main from "./pages/Main";
import Settings from "./pages/Settings";

render(
  () => (
    <main class="h-screen w-full">
      <Router>
        <Route path="/" component={Main} />
        <Route path="/settings" component={Settings} />
      </Router>
    </main>
  ),
  document.getElementById("root") as HTMLElement,
);
