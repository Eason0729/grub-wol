import { LitElement, html,unsafeCSS, PropertyValueMap} from 'lit'
import { customElement, property, state } from 'lit/decorators.js'

@customElement('bread-crumb')
export class BreadCrumb extends LitElement {
  @property()
  path: string[]=["login","machine list"]
  backward(i:number){
    const backward = () =>{
      this.path=this.path.slice(0, i)
      let event=new CustomEvent("PreviousPath",{detail:{path:this.path}})
      this.dispatchEvent(event)
    }
    return backward
  }
  render() {
    let t=[],i=0;
    for (const seg of this.path) {
      i++;
      t.push(html`<li @click=${this.backward(i)} ><a href="#">${seg}</a></li>`);
    }

    return html`
    <link rel="stylesheet" href="/src/bulma.css">
    <br>
    <nav class="level">
      <div class="level-left">
        <div class="level-item">
          <h4 class="title is-4">Grub-Wol</h4>
        </div>
      </div>
      <div class="level-right">
        <div class="level-item">
          <nav class="breadcrumb is-right is-medium" aria-label="breadcrumbs">
            <ul>
            ${t}
            </ul>
          </nav>
        </div>
      </div>
    </nav>
    <br>`
  }
}