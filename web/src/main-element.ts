import { LitElement, css, html } from "lit";
import { customElement, property } from "lit/decorators.js";
import { until } from "lit/directives/until.js";

import axios from "axios";

@customElement("main-element")
export class MainElement extends LitElement {
  @property()
  login = (async function () {
    while (true) {
      let res = await axios.post("/api/get/machines", {
        withCredentials: true
      });
      if (300 > res.status && res.status >= 200) {
        break;
      } else {
        let password = prompt("Enter password to login:");
        await axios.post("/login", { password }, { withCredentials: true });
      }
    }
    // control panel goes here
    return html``;
  })();
  render() {
    return html`${until(this.login, html`<span>Loading...</span>`)}`;
  }
}
