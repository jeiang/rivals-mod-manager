/* @refresh reload */
import { render } from "solid-js/web";
import "./index.css";
import { Route, Router } from "@solidjs/router";
import Main from "./pages/Main";

render(
  () => (
    <main class="h-screen w-full">
      <Router>
        <Route path="/" component={Main} />
      </Router>
    </main>
  ),
  document.getElementById("root") as HTMLElement,
);
