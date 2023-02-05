import { LitElement, html ,css} from 'lit'
import { customElement, property, state } from 'lit/decorators.js'
import {query} from 'lit/decorators/query.js';
import { BreadCrumb } from '../component/bread_crumb';
import { MachineList } from '../component/machine_list';
import {OsList } from '../component/os_list';

@customElement('ctrl-panel')
export class Panel extends LitElement {
  @query('bread-crumb')
  bread_crumb:BreadCrumb | undefined
  @query('machine-list')
  machine_list:MachineList |undefined
  @query('os-list')
  os_list:OsList |undefined
  @property()
  os_show=false  
  @property()
  machine_show=false
  firstUpdated(){
    // run background tasks
    this.bread_crumb?.addEventListener("PreviousPath",(e)=>{
      let path=(e as any).detail.path;
      this.determinePath(path);
    })
    this.machine_list?.addEventListener("NextPath",(e)=>{
      let mac_address:number[]=(e as any).detail.mac_address

      let path=this.bread_crumb?.path

      path?.push(mac_address.map((x)=>x.toString(16)).join(":"));

      (this.bread_crumb as any).path=path;
      this.bread_crumb?.requestUpdate();
      
      this.os_list?.update_mac((e as any).detail.mac_address);

      this.determinePath(path as string[]);
    })

    this.determinePath(this.bread_crumb?.path||[]);
  }
  determinePath(path:string[]){
    switch (path.length){
      case 1:
        this.machine_show=false
        this.os_show=false
        alert("You are going to log out!")
        break
      case 2:
        this.machine_show=true
        this.os_show=false
        this.machine_list?.refresh()
        break
      case 3:
        this.machine_show=false
        this.os_show=true
        this.os_list?.refresh()
    }
  }
  render() {
    return html`
      <bread-crumb></bread-crumb>
      <machine-list ?hidden=${!this.machine_show}></machine-list>
      <os-list ?hidden=${!this.os_show}> </os-list>
    `
  }
}