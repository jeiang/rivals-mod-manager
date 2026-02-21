/* @refresh reload */
import { render } from "solid-js/web";
import "./index.css";
import { Route, Router } from "@solidjs/router";
import Main from "./pages/Main";

render(
  () => (
    <Router>
      <Route path="/" component={Main} />
    </Router>
  ),
  document.getElementById("root") as HTMLElement,
);
