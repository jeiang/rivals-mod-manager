/* @refresh reload */
import { render } from "solid-js/web";
import "./index.css";
import { Route, Router } from "@solidjs/router";
import Main from "./pages/Main";
import Settings from "./pages/Settings";
import Categories from "./pages/Categories";

render(
  () => (
    <main class="h-screen w-full">
      <Router>
        <Route path="/" component={Main} />
        <Route path="/settings" component={Settings} />
        <Route path="/categories" component={Categories} />
      </Router>
    </main>
  ),
  document.getElementById("root") as HTMLElement,
);
