import { LitElement, css, html } from "lit";
import { customElement, property } from "lit/decorators.js";
import { until } from "lit/directives/until.js";

import axios from "axios";

@customElement("main-element")
export class MainElement extends LitElement {
  @property()
  login = (async function () {
    let is_auth=false
    while(!is_auth) {
      try{
        is_auth=true
        await axios.post("/api/get/machines", {
          withCredentials: true
        });
      }catch(err){
        is_auth=false;
      }
      if (!is_auth){
        let password = prompt("Enter password to login:");
        await axios.post("/login", { password }, { withCredentials: true });
      }
    }
    // control panel goes here
    return html`
    <link rel="stylesheet" href="/src/bulma.css">
    <div class="container is-max-desktop">
      <ctrl-panel></ctrl-panel>
    </div>`;
  })();
  render() {
    return html`${until(this.login, html`<span>Loading...</span>`)}`;
  }
}
