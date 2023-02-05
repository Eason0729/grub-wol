import { LitElement, html,css,unsafeCSS } from 'lit'
import { customElement, property, state } from 'lit/decorators.js'
import axios from 'axios';

interface JsonKind {
    kind: string;
}

interface Machine {
    display_name: string;
    mac_address: number[];
    state: JsonKind;
}

@customElement('machine-list')
export class MachineList extends LitElement {
  @property()
  machine_list: Machine[] = [{display_name:"loading",mac_address:[0,0,0,0,0,0],state:{kind:"Uninited"}}];
  async refresh(){
    let res=await axios.post("/api/get/machines", { }, { withCredentials: true })
    this.machine_list=res.data.machines
  }
  select(mac_address:number[]):()=>void{
    const select=()=>{
      let event=new CustomEvent("NextPath",{detail:{mac_address}});
      this.dispatchEvent(event);
    }
    return select
  }
  render() {
    // trigger event of refreshing machine_list (running in background)
    return html`
    <link rel="stylesheet" href="/src/bulma.css">
    <article class="panel is-success is-fullwidth">
      <p class="panel-heading">
        Machine List
      </p>
      <div class="panel-block">
        <p class="control">
          <button class="button is-success is-fullwidth" @click=${this.refresh}>Refresh</button>
        </p>
      </div>
      ${this.machine_list.map((machine)=>html`
      <a class="panel-block is-active" @click=${this.select(machine.mac_address)} >
        ${machine.display_name||"Possibly Uninited"}&nbsp<span class="tag is-info is-light">${machine.mac_address.map((x)=>x.toString(16)).join(":")}</span>
      </a>
      `)}
    </article>
    `
  }
}