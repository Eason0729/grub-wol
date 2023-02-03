import { LitElement, html } from 'lit'
import { customElement, property } from 'lit/decorators.js'

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

@customElement('machine')
export class MainElement extends LitElement {
  // must set beforehand
  mac_address:number[] | undefined
  @property()
  info: Machine={
    display_name: 'loading',
    mac_address: [0,0,0,0,0,0],
    state: {kind:"Uninited"}
  };
  @property()
  os_list: Os[] = [];
  boot(os:number):any{
    let mac_address=this.info.mac_address;
    async function handler(): Promise<void>{
      let res=await axios.post("/api/op/boot", { mac_address,os }, { withCredentials: true });
      switch (res.data.kind){
        case "Success":
          alert("boot success")
          break
        case "Fail"||"NotFound":
          alert("boot fail")
          break
      }
    }
    handler
  }
  async refresh(){
    let mac_address=this.mac_address
    let res=await axios.post("/api/get/machine", {mac_address}, { withCredentials: true })
    this.info=res.data
    // TODO: render #os_id as online
  }
  render() {
    // trigger event of fetching os_list (running in background)
    let mac_address=this.info.mac_address
    axios.post("/api/get/oss", { mac_address }, { withCredentials: true }).then((res)=>{
      this.os_list=[...this.os_list,...res.data.oss];
    })
    // refresh system_status (running in background)
    this.refresh()
    return html`
    <div class="block">
      <div class="field is-grouped">
        <div class="control is-expanded">
          <input class="input" type="text" value=${this.info.display_name} disable>
        </div>
        <div class="control">
          <button class="button is-primary">Save</button>
        </div>
      </div>
    <div>
      <table>
        <tbody>
          <tr>
            <td>Mac Address</td>
            <td>0x${this.info.mac_address.map((x)=>x.toString(16))}</td>
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
          <tr>
            <td>Refresh</td>
            <td><input type=button value=refresh @click=${this.refresh}/></td>
          </tr>
        </tbody>
      </table>
    </div>
    <div>
      <table class="table">
        <thead>
          <tr>
            <th>display name</th>
            <th>select os</th>
            <th>id</th>
          </tr>
        </thead>
        <tbody>
          ${this.os_list.map((os)=>html`
          <tr id=os_${os.id}>
            <th>${os.display_name}</th>
            <th><input @click=${this.boot(os.id)} type=button value=select alt=${"click to boot into"+os.display_name}/></th>
            <th>${os.id}</th>
          </tr>
          `)}
        </tbody>
      </table>
    </div>
  </div>`
  }
}