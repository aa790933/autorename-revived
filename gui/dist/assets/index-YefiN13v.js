import{g as oe,i as y,a as ae,b as V}from"./tauri-api-CDaqG3ye.js";(function(){const t=document.createElement("link").relList;if(t&&t.supports&&t.supports("modulepreload"))return;for(const s of document.querySelectorAll('link[rel="modulepreload"]'))n(s);new MutationObserver(s=>{for(const i of s)if(i.type==="childList")for(const o of i.addedNodes)o.tagName==="LINK"&&o.rel==="modulepreload"&&n(o)}).observe(document,{childList:!0,subtree:!0});function r(s){const i={};return s.integrity&&(i.integrity=s.integrity),s.referrerPolicy&&(i.referrerPolicy=s.referrerPolicy),s.crossOrigin==="use-credentials"?i.credentials="include":s.crossOrigin==="anonymous"?i.credentials="omit":i.credentials="same-origin",i}function n(s){if(s.ep)return;s.ep=!0;const i=r(s);fetch(s.href,i)}})();function le(){const e=oe();document.getElementById("btn-minimize")?.addEventListener("click",()=>{e.minimize()}),document.getElementById("btn-maximize")?.addEventListener("click",()=>{e.toggleMaximize()}),document.getElementById("btn-close")?.addEventListener("click",()=>{e.close()})}function ce(){return window.matchMedia("(prefers-color-scheme: dark)").matches?"dark":"light"}function I(e){document.documentElement.classList.toggle("dark",e==="dark"),localStorage.setItem("theme",e)}function de(){const e=localStorage.getItem("theme");I(e??ce()),window.matchMedia("(prefers-color-scheme: dark)").addEventListener("change",t=>{localStorage.getItem("theme")||I(t.matches?"dark":"light")})}function ue(){const e=document.documentElement.classList.contains("dark")?"dark":"light";I(e==="dark"?"light":"dark")}const R=new Set;let h={view:"files",files:[],processing:!1,progress:"",lastResult:null,dryRunResult:null,statusError:"",lastBatchId:null};function E(){return h}function c(e){h={...h,...e},R.forEach(t=>t(h))}function A(e){return R.add(e),()=>R.delete(e)}function w(e){const t=new Set(h.files.map(n=>n.path)),r=e.filter(n=>!t.has(n)).map(n=>({path:n,name:n.split(/[\\/]/).pop()||n,status:"pending"}));c({files:[...h.files,...r],dryRunResult:null,lastResult:null})}function G(){c({files:[],dryRunResult:null,lastResult:null,progress:"",statusError:"",lastBatchId:null})}function N(e){return e.replace(/\\/g,"/").toLowerCase()}function P(e,t=!1){const r=new Map(e.files.map(s=>[N(s.file),s])),n=h.files.map(s=>{const i=r.get(N(s.path));return i?{...s,status:t&&i.status==="renamed"?s.status:i.status,result:i}:s});c({files:n})}async function W(e={}){return typeof e=="object"&&Object.freeze(e),await y("plugin:dialog|open",{options:e})}var O;(function(e){e[e.Start=0]="Start",e[e.Current=1]="Current",e[e.End=2]="End"})(O||(O={}));async function pe(e,t){if(e instanceof URL&&e.protocol!=="file:")throw new TypeError("Must be a file URL.");return await y("plugin:fs|read_dir",{path:e instanceof URL?e.toString():e,options:t})}async function fe(e,t){if(e instanceof URL&&e.protocol!=="file:")throw new TypeError("Must be a file URL.");const r=await y("plugin:fs|read_text_file",{path:e instanceof URL?e.toString():e,options:t}),n=r instanceof ArrayBuffer?r:Uint8Array.from(r);return new TextDecoder().decode(n)}async function ge(e,t,r){if(e instanceof URL&&e.protocol!=="file:"||t instanceof URL&&t.protocol!=="file:")throw new TypeError("Must be a file URL.");await y("plugin:fs|rename",{oldPath:e instanceof URL?e.toString():e,newPath:t instanceof URL?t.toString():t,options:r})}async function me(e,t,r){if(e instanceof URL&&e.protocol!=="file:")throw new TypeError("Must be a file URL.");const n=new TextEncoder;await y("plugin:fs|write_text_file",n.encode(t),{headers:{path:encodeURIComponent(e instanceof URL?e.toString():e),options:JSON.stringify(r)}})}const M=[".pdf",".docx",".xlsx",".png",".jpg",".jpeg",".tiff",".tif",".bmp"];function m(e){return e.replace(/&/g,"&amp;").replace(/</g,"&lt;").replace(/>/g,"&gt;").replace(/"/g,"&quot;").replace(/'/g,"&#39;")}function ye(e){return M.some(t=>e.toLowerCase().endsWith(t))}async function Z(e){const t=e.replace(/[\\/]+$/,""),r=t.includes("\\")?"\\":"/";return(await pe(t)).filter(s=>s.isFile&&ye(s.name)).map(s=>t+r+s.name)}async function C(){const e=await W({multiple:!0,filters:[{name:"Supported Documents",extensions:M.map(t=>t.slice(1))}]});return e?Array.isArray(e)?e:[e]:[]}async function be(){const e=await W({directory:!0,multiple:!1});return e?Z(e):null}function $(e){const t=e.toLowerCase().replace(/.*[.](\w+)$/,".$1");return M.includes(t)}function ve(e,t){let r;return ae().onDragDropEvent(async n=>{switch(n.payload.type){case"over":t(!0);break;case"drop":{t(!1);try{const s=n.payload.paths,i=s.filter(d=>$(d)),o=s.filter(d=>!$(d)),a=await Promise.all(o.map(async d=>{try{return await Z(d)}catch{return[]}})),f=[...i,...a.flat()].filter(d=>$(d));e(f)}catch{e([])}break}case"leave":t(!1);break}}).then(n=>{r=n}),()=>r?.()}function he(e){return!e.success&&"error_type"in e}async function we(e,t={}){try{return await y("rename_pdfs",{paths:e,options:t})}catch(r){return{success:!1,error_type:"sidecar_error",message:String(r),suggestion:""}}}async function ke(e){try{return await y("undo_rename",{batchId:e})}catch(t){throw new Error(String(t))}}async function xe(){return await V()}async function Ee(){try{return await y("validate_config")}catch(e){throw new Error(String(e))}}function _e(e,t){const r=e.includes("\\")?"\\":"/";return e+r+t}function Le(){return Array.from(crypto.getRandomValues(new Uint8Array(4))).map(e=>e.toString(16).padStart(2,"0")).join("")}async function $e(){const e=await V();return _e(e,"rename_history.json")}async function Se(e){try{const t=JSON.parse(await fe(e));return Array.isArray(t)?{version:2,batches:t.length>0?[{batch_id:"migrated-v1",timestamp:t[0]?.timestamp??"",source:"cli",undone:!1,files:t}]:[]}:t}catch{return{version:2,batches:[]}}}async function Ie(e,t,r){const n=[];let s=0,i=0,o=0;const a=[],f=Le(),d=e.length;for(let p=0;p<e.length;p++){const g=e[p],b=g.result;if(!b?.new_path||b.status==="skipped"){i++,n.push(b?{...b,status:"skipped"}:{file:g.path,status:"skipped",new_name:null,new_path:null,error:null,warnings:[],company:null,date:null,doc_type:null,provider:null,model:null});continue}try{await ge(g.path,b.new_path),s++,n.push({...b,status:"renamed"}),a.push({old_path:g.path,new_path:b.new_path,timestamp:new Date().toISOString()})}catch(ie){o++,n.push({...b,status:"failed",error:String(ie)})}}if(a.length>0){const p=await $e();try{const g=await Se(p);g.batches.push({batch_id:f,timestamp:new Date().toISOString(),source:"gui",undone:!1,files:a}),await me(p,JSON.stringify(g,null,2))}catch{}}return{success:o===0,total:d,renamed:s,skipped:i,failed:o,files:n,dry_run:!1,batch_id:f}}const B={ai:{provider:"gemini",api_key:"",model:"gpt-4o-mini",gemini_model:"gemini-3.1-flash-lite",gemini_api_key:"",gemini_base_url:"",custom_model:"",custom_base_url:"",temperature:0,timeout:30},pdf:{vision:"auto",vision_provider:"gemini"},naming:{template:"{date}_{company}_{doctype}",fallback:"{date}_Unknown_{doctype}",date_format:"%Y%m%d",separator:"_",max_length:128,sequence_zerofill:2},undo:{enabled:!0,log_path:"~/.autorename-revived/rename_history.json",max_entries:100},debug:!1,max_workers:4,harmonized_companies:[]};let v=structuredClone(B),T=!1;async function J(){if(T)return v;try{const e=await y("load_app_config");v={...structuredClone(B),...e}}catch{v=structuredClone(B)}return T=!0,v}function Re(){return v}async function Ce(e){v=structuredClone(e),T=!0;try{await y("save_app_config",{config:v})}catch(t){console.warn("Failed to persist config via Rust backend:",t)}}const Be={success:"✓",danger:"✗",warning:"!",info:"ℹ"};function l(e,t="info",r=5e3){const n=document.querySelector(".toast-container");if(!n)return;const s=document.createElement("div");s.className=`toast toast-${t} toast-entry-right`,s.innerHTML=`
    <span class="toast-icon">${Be[t]}</span>
    <span class="toast-message">${m(e)}</span>
    <button class="toast-close" aria-label="Dismiss">&times;</button>
  `,s.querySelector(".toast-close")?.addEventListener("click",()=>s.remove()),n.appendChild(s),requestAnimationFrame(()=>s.classList.add("show")),r>0&&setTimeout(()=>{s.classList.add("hiding"),s.classList.remove("show"),setTimeout(()=>s.remove(),300)},r)}let k,Y,F;function Te(e){k=e,Y=ve(t=>{t.length>0?w(t):l("No PDF files in drop","warning")},t=>{const r=k.querySelector(".drop-zone"),n=k.querySelector("#file-list-container");r?r.classList.toggle("drop-zone-active",t):n&&n.classList.toggle("drag-hover",t)}),F=A(j),j(E())}function Fe(){F?.(),F=void 0,Y?.()}function j(e){e.view==="files"&&(e.files.length===0?Ae():Ue(e))}function Ae(){k.innerHTML=`
    <div class="flex flex-col items-center justify-center flex-1 p-8">
      <div class="drop-zone flex flex-col items-center justify-center gap-4 p-12 w-full max-w-lg
                  border-2 border-dashed rounded-xl border-[var(--border-secondary)]
                  hover:border-[var(--color-primary)] transition-colors cursor-pointer"
           id="drop-zone-area">
        <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor"
             stroke-width="1.5" class="text-[var(--text-tertiary)]">
          <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/>
          <polyline points="14 2 14 8 20 8"/>
          <line x1="12" y1="18" x2="12" y2="12"/>
          <line x1="9" y1="15" x2="12" y2="12"/>
          <line x1="15" y1="15" x2="12" y2="12"/>
        </svg>
        <p class="text-[var(--text-secondary)] text-center">
          Drop PDF files here<br>
          <span class="text-sm text-[var(--text-tertiary)]">or click to browse</span>
        </p>
        <div class="flex gap-3 mt-2">
          <button class="btn btn-primary btn-sm" id="btn-browse-files">Browse Files</button>
          <button class="btn btn-secondary btn-sm" id="btn-browse-folder">Browse Folder</button>
        </div>
      </div>
    </div>
  `,document.getElementById("btn-browse-files")?.addEventListener("click",async()=>{const e=await C();e.length>0&&w(e)}),document.getElementById("btn-browse-folder")?.addEventListener("click",async()=>{const e=await be();e!==null&&(e.length>0?w(e):l("No supported files found in folder","warning"))}),document.getElementById("drop-zone-area")?.addEventListener("click",async e=>{if(e.target.closest("button"))return;const t=await C();t.length>0&&w(t)})}function Me(e){return e.status==="pending"&&e.result?.new_name?"preview":e.status}function qe(e){const t=Me(e),r=e.result?.new_name,n=e.result?.error,s=`fq-dot fq-dot-${t}`,i=`fq-badge fq-badge-${t}`,o=t==="preview"?"preview":e.status;let a="";r&&(t==="preview"||t==="renamed")&&(a=`<span class="fq-preview"><span class="fq-arrow">→</span><span class="fq-new-name ${t==="preview"?"fq-new-name-preview":"fq-new-name-renamed"}">${m(r)}</span></span>`),n&&t==="failed"&&(a=`<span class="fq-error">${m(n)}</span>`);const f=e.result?.warnings??[];return f.length>0&&t!=="failed"&&(a+=`<span class="fq-warning">⚠ ${f.map(m).join("; ")}</span>`),`
    <div class="fq-row">
      <span class="${s}"></span>
      <div class="fq-info">
        <span class="fq-name">${m(e.name)}</span>
        ${a}
      </div>
      <span class="${i}">${o}</span>
    </div>`}function Ue(e){const t=e.lastResult!==null,r=e.files.filter(i=>i.status==="pending").length,n=e.processing;let s;if(t)s=`
      <div class="fq-actions-left">
        <button class="btn btn-secondary btn-sm" id="btn-undo" ${n?"disabled":""}>Undo Last</button>
        <button class="btn btn-primary btn-sm" id="btn-add-more" ${n?"disabled":""}>Add More Files</button>
      </div>`;else{const o=n||r===0||!!e.statusError;s=`
      <div class="fq-actions-left">
        <button class="btn btn-secondary btn-sm" id="btn-dry-run" ${o?"disabled":""}>Dry Run</button>
        <button class="btn btn-primary btn-sm" id="btn-rename" ${o?"disabled":""}>
          Rename ${r} File${r!==1?"s":""}
        </button>
      </div>`}k.innerHTML=`
    <div class="fq-container" id="file-list-container">
      <div class="fq-header">
        <span class="fq-count">${e.files.length} file${e.files.length!==1?"s":""}</span>
        ${n?`<span class="fq-progress-text">${e.progress||"Processing…"}</span>`:""}
      </div>
      ${n?'<div class="fq-progress-bar"></div>':""}
      <div class="fq-list">
        ${e.files.map(qe).join("")}
      </div>
      <div class="fq-actions">
        ${s}
        <button class="btn btn-ghost btn-sm" id="btn-clear" ${n?"disabled":""}>Clear</button>
      </div>
    </div>
  `,document.getElementById("btn-dry-run")?.addEventListener("click",()=>z(!0)),document.getElementById("btn-rename")?.addEventListener("click",()=>z(!1)),document.getElementById("btn-clear")?.addEventListener("click",()=>G()),document.getElementById("btn-undo")?.addEventListener("click",De),document.getElementById("btn-add-more")?.addEventListener("click",async()=>{const i=await C();i.length>0&&w(i)})}async function z(e){const t=E();if(!e&&t.dryRunResult){const i=t.files.filter(o=>o.status==="pending"||o.status==="skipped");if(i.length===0){l("No files to process","warning");return}c({processing:!0,progress:"Applying cached results…"});try{const o=await xe(),a=await Ie(i,o);c({processing:!1,progress:"",statusError:""}),P(a,!1),c({lastResult:a,dryRunResult:null,lastBatchId:a.batch_id??null}),a.failed>0?l(`${a.renamed} renamed, ${a.failed} failed`,"warning"):l(`${a.renamed} files renamed successfully`,"success")}catch(o){c({processing:!1,progress:""}),l(`Rename failed: ${o}`,"danger")}return}const r=t.files.filter(i=>i.status==="pending"||i.status==="skipped").map(i=>i.path);if(r.length===0){l("No files to process","warning");return}c({processing:!0,progress:"Starting..."});let n;try{n=await we(r,{dryRun:e,provider:Re().ai.provider})}catch(i){const o=String(i),a=o.toLowerCase();a.includes("sidecar")||a.includes("not found")||a.includes("binaries")||a.includes("os error")||a.includes("cannot find")||a.includes("introuvable")||a.includes("no such file")?(c({processing:!1,progress:"",statusError:"CLI executable not found"}),l('CLI executable not found. Re-extract the portable ZIP, or run "python build.py --cli-only --nosign" if developing.',"danger")):(c({processing:!1,progress:""}),l(`Error: ${o}`,"danger"));return}if(c({processing:!1,progress:"",statusError:""}),he(n)){let i=n.message,o="";n.error_type==="sidecar_error"?(i='CLI executable not found. Re-extract the portable ZIP, or run "python build.py --cli-only --nosign" if developing.',o="CLI executable not found"):n.error_type==="config_error"?(i="config.yaml missing or invalid — copy config.yaml.example and add your API key",o="Config error"):n.error_type==="auth_error"&&(i="API key missing or invalid — set ai.api_key in config.yaml",o="Auth error"),n.suggestion&&(i+=`. ${n.suggestion}`),o&&c({statusError:o}),l(i,"danger");return}const s=n;if(P(s,e),e)c({dryRunResult:s}),s.renamed===0&&s.skipped>0?l("Preview: all files already correctly named","info"):l(`Preview: ${s.renamed} to rename, ${s.skipped} to skip`,"info");else{s.renamed>0?c({lastResult:s,lastBatchId:s.batch_id??null}):c({lastResult:s,lastBatchId:null}),s.failed>0?l(`${s.renamed} renamed, ${s.failed} failed`,"warning"):s.renamed===0&&s.skipped>0?l("All files already correctly named","info"):l(`${s.renamed} files renamed successfully`,"success");const i=s.files.flatMap(a=>a.warnings??[]),o=[...new Set(i)];for(const a of o)l(a,"warning")}}async function De(){const{lastBatchId:e}=E();if(!e){l("Nothing to undo","info");return}c({processing:!0,progress:"Undoing..."});try{const t=await ke(e??void 0);if(c({processing:!1,progress:""}),"error_type"in t){let r=t.message;t.suggestion&&(r+=`. ${t.suggestion}`),l(r,"danger")}else t.success?(l(`${t.restored} files restored`,"success"),G()):l(`Undo: ${t.restored} restored, ${t.failed} failed`,"warning")}catch(t){c({processing:!1,progress:""}),l(`Undo failed: ${t}`,"danger")}}const x={gemini:{label:"Google Gemini",icon:'<svg viewBox="0 0 24 24" fill="currentColor" class="w-4 h-4"><path d="M12 2L2 19.5h20L12 2zm0 4l6.5 11.5h-13L12 6z"/></svg>',fields:[{key:"api_key",label:"API Key",type:"secret",configKey:"ai"},{key:"gemini_model",label:"Text Model",type:"string",hint:"e.g. gemini-2.0-flash",configKey:"ai"},{key:"gemini_base_url",label:"Base URL (optional)",type:"string",configKey:"ai"}]},openai:{label:"OpenAI",icon:'<svg viewBox="0 0 24 24" fill="currentColor" class="w-4 h-4"><circle cx="12" cy="12" r="10"/></svg>',fields:[{key:"api_key",label:"API Key",type:"secret",configKey:"ai"},{key:"model",label:"Model",type:"string",hint:"e.g. gpt-4o-mini",configKey:"ai"}]},custom:{label:"Custom / Ollama / vLLM",icon:'<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class="w-4 h-4"><path d="M12 2v20M2 12h20"/></svg>',fields:[{key:"api_key",label:"API Key",type:"secret",configKey:"ai"},{key:"custom_model",label:"Model",type:"string",hint:"e.g. llama3.2, qwen-72b",configKey:"ai"},{key:"custom_base_url",label:"Base URL",type:"string",hint:"e.g. http://localhost:11434/v1",configKey:"ai"}]}},X=[{key:"temperature",label:"Temperature",type:"number",hint:"0.0 = deterministic, 1.0 = creative",configKey:"ai"},{key:"timeout",label:"Timeout (seconds)",type:"number",configKey:"ai"}],Q=[{key:"vision",label:"Vision (scanned docs)",type:"auto-or-bool",hint:"auto = use AI vision for scanned pages",configKey:"pdf"}],ee=[{key:"template",label:"Filename Template",type:"string",hint:"{date}, {company}, {doctype}, {category}, {subject}, {original}, {sequence}",configKey:"naming"},{key:"fallback",label:"Fallback Template",type:"string",hint:"Used when template yields empty name",configKey:"naming"},{key:"date_format",label:"Date Format",type:"string",hint:"strftime format, e.g. %Y%m%d",configKey:"naming"},{key:"separator",label:"Separator",type:"string",configKey:"naming"},{key:"max_length",label:"Max Filename Length",type:"number",configKey:"naming"},{key:"sequence_zerofill",label:"Sequence Zero-Fill",type:"number",hint:"Pad sequence numbers to this width",configKey:"naming"}],te=[{key:"enabled",label:"Enable Undo",type:"toggle",configKey:"undo"},{key:"log_path",label:"Log Path",type:"string",hint:"Path to rename history log",configKey:"undo"},{key:"max_entries",label:"Max Entries",type:"number",configKey:"undo"}],ne=[{key:"debug",label:"Debug Mode",type:"toggle",configKey:"_general"},{key:"max_workers",label:"Max Workers",type:"number",hint:"Parallel rename threads",configKey:"_general"}];function q(e,t){return t.split(".").reduce((r,n)=>{if(r&&typeof r=="object")return r[n]},e)}function se(e){return e==="auto"?"auto":e?"true":"false"}function Ke(e){return e==="auto"||e===""||e===void 0?e:e.toLowerCase()}function Ne(e,t,r){const n=String(t??""),s=`field-${r.replace(/\./g,"-")}`;switch(e.type){case"secret":return`
        <div class="input-group">
          <input id="${s}" type="password" class="input input-sm" value="${m(n)}" autocomplete="off">
          <button type="button" class="btn-toggle-password btn btn-ghost btn-sm" aria-label="Toggle visibility">
            <svg class="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path class="toggle-eye-closed" d="M10.585 10.585a2 2 0 102.83 2.83 2 2 0 00-2.83-2.83z"/>
              <path class="toggle-eye-open" style="display:none" d="M1 12s8 6 11 6 11-6 11-6-8-6-11-6S1 12 1 12z"/>
            </svg>
          </button>
        </div>`;case"auto-or-bool":{const i=["auto","true","false"].map(o=>`<option value="${o}" ${o===se(t)?"selected":""}>${o}</option>`).join("");return`<select id="${s}" class="input input-sm">${i}</select>`}case"number":return`<input id="${s}" type="number" class="input input-sm" value="${m(n)}">`;case"toggle":return`<input id="${s}" type="checkbox" class="checkbox toggle-checkbox" ${!!t?"checked":""}>`;case"string":default:return`<input id="${s}" type="text" class="input input-sm" value="${m(n)}" autocomplete="off">`}}function re(e,t,r){const n=e.hint?`<div class="form-hint">${m(e.hint)}</div>`:"";return`
    <div class="settings-field">
      <label class="label">${m(e.label)}</label>
      ${Ne(e,t,r)}
      ${n}
    </div>`}function Pe(e){const t=e.ai.provider||"gemini";return[...(x[t]||x.gemini).fields,...X].map(s=>{const i=s.configKey==="_general"?e[s.key]:q(e[s.configKey]||{},s.key);return re(s,i,`${s.configKey}.${s.key}`)}).join("")}function _(e,t,r){const n=t.map(s=>{const i=s.configKey==="_general"?r[s.key]:q(r[s.configKey]||{},s.key);return re(s,i,`${s.configKey}.${s.key}`)}).join("");return n?`
    <div class="settings-section">
      <h3>${m(e)}</h3>
      <div class="card card-bordered">${n}</div>
    </div>`:""}let u=null;async function Oe(e){e.innerHTML=`
    <div class="flex flex-col flex-1 min-h-0 p-6 pt-8">
      <div class="flex items-center justify-between mb-4">
        <h2 class="text-lg font-semibold">Settings</h2>
        <button class="btn btn-secondary btn-sm" id="btn-back-from-settings">
          <svg class="w-3.5 h-3.5 inline-block mr-1 -mt-px" viewBox="0 0 24 24" fill="none" stroke="currentColor"
               stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <polyline points="15 18 9 12 15 6"/>
          </svg>Back
        </button>
      </div>
      <div id="provider-bar" class="provider-bar mb-4"></div>
      <div id="settings-content" class="flex-1 overflow-y-auto" style="overflow-x:hidden;padding-right:0.5rem">
      </div>
      <div class="flex items-center justify-center gap-2 mt-4" id="settings-footer">
        <button class="btn btn-secondary btn-sm" id="btn-save-settings">Save All</button>
        <button class="btn btn-secondary btn-sm" id="btn-test-connection">Test Connection</button>
      </div>
    </div>
  `,document.getElementById("btn-back-from-settings")?.addEventListener("click",()=>{c({view:"files"})}),document.getElementById("btn-test-connection")?.addEventListener("click",async()=>{const r=document.getElementById("btn-test-connection");r&&(r.disabled=!0,r.textContent="Testing...");try{const n=document.getElementById("field-ai-provider"),s=document.getElementById("field-ai-api_key"),i=document.getElementById("field-ai-model"),o=n?.value||u?.ai.provider||"gemini",a=s?.value||"",f=i?.value||"",d=await y("test_connection",{provider:o,apiKey:a,model:f});l(`${d.provider}: ${d.message}`,d.success?"success":"danger")}catch(n){l(`Connection test failed: ${n}`,"danger")}finally{r&&(r.disabled=!1,r.textContent="Test Connection")}}),document.getElementById("btn-save-settings")?.addEventListener("click",async()=>{await je()});const t=await J();c({statusError:""}),u=t,U(t),D(t),K()}function U(e){const t=document.getElementById("provider-bar");if(!t)return;const r=e.ai.provider||"gemini";t.innerHTML=`<div class="provider-bar-inner">
    ${Object.entries(x).map(([n,s])=>`<button class="provider-btn${n===r?" provider-btn-active":""}" data-provider="${n}">
        <span class="provider-btn-icon">${s.icon}</span>
        <span class="provider-btn-label">${s.label}</span>
      </button>`).join("")}
  </div>`,t.querySelectorAll(".provider-btn").forEach(n=>{n.addEventListener("click",()=>{const s=n.dataset.provider;!s||!u||(u.ai.provider=s,U(u),D(u),K())})})}function D(e){const t=document.getElementById("settings-content");if(!t)return;const r=e.ai.provider||"gemini",n=Pe(e),s=_("Document Processing",Q,e),i=_("File Naming",ee,e),o=_("Undo History",te,e),a=_("General",ne,e);t.innerHTML=`
    <div class="space-y-4">
      <div class="settings-section">
        <h3>${m(x[r]?.label||r)}</h3>
        <div class="card card-bordered">${n}</div>
      </div>
      ${s}
      ${i}
      ${o}
      ${a}
    </div>
  `}function K(){document.querySelectorAll(".btn-toggle-password").forEach(e=>{e.addEventListener("click",()=>{const t=e.closest(".input-group")?.querySelector("input");t&&(t.type=t.type==="password"?"text":"password")})})}async function je(){if(!u){l("No configuration loaded","warning");return}const e=[],t=[...Object.values(x).flatMap(n=>n.fields),...X,...Q,...ee,...te,...ne],r=new Set;for(const n of t){const s=`${n.configKey}.${n.key}`;if(r.has(s))continue;r.add(s);const i=`field-${s.replace(/\./g,"-")}`,o=document.getElementById(i);if(!o)continue;let a;if(o instanceof HTMLInputElement&&o.type==="checkbox")a=o.checked?"true":"false";else if(o instanceof HTMLInputElement||o instanceof HTMLSelectElement)a=o.value;else continue;let f;if(n.configKey==="_general")f=String(u[n.key]??"");else{const g=u[n.configKey]||{};f=String(q(g,n.key)??"")}const d=n.type==="auto-or-bool"?se(f):f,p=n.type==="auto-or-bool"?Ke(a):a;if(d!==p)if(e.push({key:s,value:p}),n.configKey==="_general")u[n.key]=n.type==="toggle"?p==="true":n.type==="number"?Number(p):p;else{const g=u[n.configKey];g&&(g[n.key]=n.type==="toggle"?p==="true":n.type==="number"?Number(p):p)}}if(e.push({key:"ai.provider",value:u.ai.provider}),!e.length){l("No changes to save","info");return}try{await Ce(u);const n=await y("save_app_config_batch",{updates:e});n.failed>0&&n.saved>0?l(`${n.saved} saved, ${n.failed} failed`,"warning"):n.failed>0?l(`Save failed: ${n.errors?.[0]||"Unknown error"}`,"danger"):l(`${n.saved} settings saved`,"success"),U(u),D(u),K()}catch(n){l(`Save failed: ${n}`,"danger")}}function ze(){u=null}async function He(e,t={},r){return window.__TAURI_INTERNALS__.invoke(e,t,r)}async function Ve(e,t){await He("plugin:opener|open_url",{url:e,with:t})}const Ge="https://github.com/aa790933/autorename-revived",We="https://github.com/aa790933/autorename-revived/blob/main/LICENSE",Ze="https://phrasevault.app",S='<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6"/><polyline points="15 3 21 3 21 9"/><line x1="10" y1="14" x2="21" y2="3"/></svg>';function Je(e){const t="3.0.4";e.innerHTML=`
    <div class="about-scroll">
    <div class="about-view">

      <div class="about-header">
        <h1>AutoRename-Revived</h1>
        <p class="about-tagline">
          AI-powered PDF auto-renamer.<br>
          Extracts company name, date, and document type from PDFs.
        </p>
        <p class="about-version">v${t}</p>
      </div>

      <div class="about-section">
        <p class="about-section-title">Open Source</p>
        <p>
          Licensed under the
          <button class="about-link" data-href="${We}">MIT License ${S}</button>
        </p>
        <p style="margin-top: 0.5rem;">
          <button class="about-link" data-href="${Ge}">View on GitHub ${S}</button>
        </p>
      </div>

      <div class="about-section">
        <p class="about-section-title">Technology</p>
        <p style="font-size: var(--font-size-xs, 0.75rem); color: var(--text-secondary); line-height: 1.6;">
          Tauri v2 desktop app with TypeScript frontend and Rust backend.<br>
          AI metadata extraction via Python CLI sidecar with multi-provider support.
        </p>
      </div>

      <div class="about-section">
        <p class="about-section-title">Disclaimer</p>
        <p>
          This software is provided "as is", without warranty of any kind, express or implied.
          The authors are not liable for any claim, damages, or other liability arising from its use.
        </p>
      </div>

      <div class="about-support-card">
        <p>
          If you find this project useful, please consider supporting its development by checking out
        </p>
        <p>
          <button class="about-link" data-href="${Ze}" style="font-size:var(--font-size-sm);">PhraseVault ${S}</button>
        </p>
        <p>
          A text expander and snippet manager by the same developer. Your support helps keep this project free and maintained.
        </p>
      </div>

      <button class="btn btn-ghost btn-sm" id="btn-back-from-about">&larr; Back</button>
    </div>
    </div>
  `,e.querySelectorAll("[data-href]").forEach(r=>{r.addEventListener("click",()=>{const n=r.dataset.href;n&&Ve(n)})}),document.getElementById("btn-back-from-about")?.addEventListener("click",()=>{c({view:"files"})})}let L=null;function Ye(e){A(t=>{t.view!==L&&H(e,t.view)}),H(e,E().view)}function H(e,t){switch(L==="files"&&Fe(),L==="settings"&&ze(),L=t,t){case"files":Te(e);break;case"settings":Oe(e);break;case"about":Je(e);break}}function Xe(e){const t=document.createElement("div");t.className="status-bar",t.innerHTML=`
    <div class="status-bar-left">
      <button class="status-bar-btn" data-view="settings" title="Settings" aria-label="Settings">
        <svg class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z"/>
          <circle cx="12" cy="12" r="3"/>
        </svg>
      </button>
      <button id="btn-toggle-theme" class="status-bar-btn" title="Toggle theme">
        <svg class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <circle cx="12" cy="12" r="5"/><line x1="12" y1="1" x2="12" y2="3"/><line x1="12" y1="21" x2="12" y2="23"/><line x1="4.22" y1="4.22" x2="5.64" y2="5.64"/><line x1="18.36" y1="18.36" x2="19.78" y2="19.78"/><line x1="1" y1="12" x2="3" y2="12"/><line x1="21" y1="12" x2="23" y2="12"/><line x1="4.22" y1="19.78" x2="5.64" y2="18.36"/><line x1="18.36" y1="5.64" x2="19.78" y2="4.22"/>
        </svg>
      </button>
      <button class="status-bar-btn" data-view="about" title="About" aria-label="About">
        <svg class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <circle cx="12" cy="12" r="10"/><line x1="12" y1="16" x2="12" y2="12"/><line x1="12" y1="8" x2="12.01" y2="8"/>
        </svg>
      </button>
    </div>
    <div class="status-bar-right">
      <span id="status-text">Ready</span>
    </div>
  `,e.appendChild(t),t.querySelectorAll("[data-view]").forEach(r=>{r.addEventListener("click",()=>{const n=r.dataset.view,s=E().view;c({view:s===n?"files":n})})}),document.getElementById("btn-toggle-theme")?.addEventListener("click",ue),A(r=>{const n=document.getElementById("status-text");if(n)if(r.processing)n.textContent=r.progress||"Processing...",n.classList.remove("status-error");else if(r.statusError)n.textContent=r.statusError,n.classList.add("status-error");else if(r.files.length>0){const s=r.lastResult?.files[0]?.provider??"",i=r.lastResult?.files[0]?.model??"",o=s?` · ${s}${i?` / ${i}`:""}`:"";n.textContent=`${r.files.length} files${o}`,n.classList.remove("status-error")}else n.textContent="Ready",n.classList.remove("status-error")})}document.addEventListener("DOMContentLoaded",()=>{le(),de();const e=document.getElementById("app");if(!e)throw new Error("#app element not found");const t=document.createElement("div");t.className="flex flex-col flex-1 min-h-0",e.appendChild(t),Xe(e),Ye(t),J().catch(()=>{}),Ee().then(r=>{r.valid||r.issues.filter(s=>s.level==="error").length>0&&c({statusError:"Config error"})}).catch(()=>{c({statusError:"Config error"})})});
