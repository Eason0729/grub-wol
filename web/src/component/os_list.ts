import { LitElement, html ,css,unsafeCSS} from 'lit'
import { customElement, property, state,query } from 'lit/decorators.js'
import {BreadCrumb} from './bread_crumb';
import axios from 'axios';

interface JsonKind {
    kind: string;
}

interface Machine {
    display_name: string;
    mac_address: number[];
    state: JsonKind;
}

interface Os {
    display_name: string;
    id: number;
}

@customElement('os-list')
export class OsList extends LitElement {
  @query('bread-crumb')
  bread_crumb:BreadCrumb | undefined
  @property()
  info: Machine={
    display_name: '',
    mac_address: [0,0,0,0,0,0],
    state: {kind:"Uninited"}
  };
  @property()
  os_list: Os[] = [];
  async refresh(){
    let mac_address=this.info.mac_address
    try{
      let res1=await axios.post("/api/get/machine", {mac_address}, { withCredentials: true });
      if (!res1.data){
        throw "Option<MachineInfoInner> is None"
      }
      this.info=res1.data;

      let res2=await axios.post("/api/get/oss", { mac_address }, { withCredentials: true });
      this.os_list=res2.data.oss;
      // TODO: mark active os
    }catch(err){
      console.warn("this machine is uninited");
      console.warn(err);
    }
  }
  async init(){ 
    let display_name=this.info.display_name
    let mac_address=this.info.mac_address
    axios.post("/api/op/new", { mac_address ,display_name}, { withCredentials: true })
    this.bread_crumb?.backward(2)
    // TODO: refactor update machinism
    alert("Adding new machine")
  }
  update_mac(mac_address:number[]){
    this.info.mac_address=mac_address
    this.requestUpdate()
  }
  async display_name_onchange(e:Event){
    let ele=e.target as HTMLInputElement;
    this.info.display_name=ele.value;
  }
  async shutdown(){
    let mac_address=this.info.mac_address;
    let res=await axios.post("/api/op/boot", { mac_address,os:{kind:"Down"} }, { withCredentials: true });
    switch (res.data.kind){
      case "Success":
        alert("boot success")
        break
      case "Fail"||"NotFound":
        alert("boot fail")
        break
    }
  }
  boot(os:number):any{
    let mac_address=this.info.mac_address;
    async function handler(): Promise<void>{
      let res=await axios.post("/api/op/boot", { mac_address,os:{kind:"Up",id:os} }, { withCredentials: true });
      switch (res.data.kind){
        case "Success":
          alert("boot success")
          break
        case "Fail"||"NotFound":
          alert("boot fail")
          break
      }
    }
    return handler
  }
  render() {
    // trigger event of fetching os_list (running in background)
    return html`
    <link rel="stylesheet" href="/src/bulma.css">
    <div>
      ${this.info.state.kind!="Uninited"? "" : html`
      <div class="notification is-warning is-light">
        <button class="delete"></button>
        This is possibly an uninitialized machine.
        <br>
        Enter Display Name and press <span class="tag is-warning">New Machine</span>, and <strong>Grub-Wol</strong> will try to discover bootable Operating System Within it.
        <br>
        Also of note that initializing a machine require host to scan all the disks for bootable Operating System, which should take a long time.
      </div>
      `}
      <div class="block">
        <div class="field is-grouped">
          <div class="control is-expanded">
            <input class="input" type="text" .value=${this.info.display_name} placeholder="Enter Display Name of this Machine" disable>
          </div>
          <div class="control">
            ${this.info.state.kind=="Uninited"? html`
            <button class="button is-warning" @click=${this.init}>New Machine</button>
            ` : html`
            <button class="button is-primary">Save</button>
            `}
          </div>
        </div>
      <div>
      <div class="table-container">
        <table class="table is-fullwidth">
          <thead>
            <tr>
              <th>Properties</th>
              <th>Value</th>
            </tr>
          </thead>
          <tbody>
            <tr>
              <td>Mac Address</td>
              <td>${this.info.mac_address.map((x)=>x.toString(16)).join(":")}</td>
            </tr>
            <tr>
              <td>System Status</td>
              <td>${this.info.state.kind}</td>
            </tr>
            ${this.info.state.kind=="Up"? html`
            <tr>
              <td>Operating System id</td>
              <td>${(this.info.state as any).id}</td>
            </tr>
            ` : ""}
          </tbody>
        </table>
      </div>
      ${this.info.state.kind=="Uninited"? "" : html`
      <article class="panel is-success">
        <p class="panel-heading">
          Operating System List
        </p>
        <div class="panel-block">
          <p class="control has-icons-left">
            <input class="input is-success" type="text" placeholder="Enter Display Name or ID of the Operating System">
          </p>
        </div>
        <div class="panel-block">
          <p class="control">
            <button class="button is-success is-fullwidth" @click=${this.refresh}>Refresh</button>
          </p>
        </div>
        ${this.os_list.map((os)=>html`
        <a class="panel-block" @click=${this.boot(os.id)} id=os_${os.id}>
          <input type="checkbox">
          ${os.display_name} 
          <span class="tag is-success is-light">${os.id}</span>
        </a>
        `)}
        <div class="panel-block">
          <p class="control">
            <button class="button is-warning is-fullwidth" @click=${this.shutdown}>Shutdown</button>
          </p>
        </div>
      </article>
      `}
    </div>`
  }
}