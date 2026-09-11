// Runs in the page: only user-scrollable ancestors are moved. Hidden clipping
// cannot be made to pass by programmatically scrolling an overflow:hidden box.
export function inspectGeometry() {
 const visible=e=>e.checkVisibility({visibilityProperty:true,contentVisibilityAuto:true})&&!e.closest('details:not([open]) > :not(summary)');
 const scope=[...document.querySelectorAll('[role=dialog]')].find(visible)||document;
 const root=scope===document?document.querySelector('.content-scroll'):scope;
 const failures=[],controls=[...scope.querySelectorAll('button,input,select,textarea,summary,.settings-nav a')].filter(visible);
 for(const e of controls){
  for(let a=e.parentElement;a&&a!==document.body;a=a.parentElement){const s=getComputedStyle(a),b=a.getBoundingClientRect(),r=e.getBoundingClientRect();if(s.position==='fixed')break;if(/auto|scroll/.test(s.overflowY)&&a.scrollHeight>a.clientHeight)a.scrollTop+=(r.top+r.height/2-b.top-b.height/2)/(b.height/a.offsetHeight||1);if(/auto|scroll/.test(s.overflowX)&&a.scrollWidth>a.clientWidth)a.scrollLeft+=(r.left+r.width/2-b.left-b.width/2)/(b.width/a.offsetWidth||1);}
  const r=e.getBoundingClientRect();let l=0,t=0,b=innerHeight,right=innerWidth;
  for(let a=e.parentElement;a&&a!==document.body;a=a.parentElement){const s=getComputedStyle(a),x=a.getBoundingClientRect();if(/auto|scroll|hidden|clip/.test(s.overflowY)){t=Math.max(t,x.top);b=Math.min(b,x.bottom);}if(/auto|scroll|hidden|clip/.test(s.overflowX)){l=Math.max(l,x.left);right=Math.min(right,x.right);}if(s.position==='fixed')break;}
  const hit=document.elementFromPoint(r.x+r.width/2,r.y+r.height/2);
  if(e instanceof HTMLInputElement&&['checkbox','radio'].includes(e.type)&&parseFloat(getComputedStyle(e).width)>26)failures.push({control:e.id||e.type,reason:'Choice control stretched like a text input',width:parseFloat(getComputedStyle(e).width)});
  if(r.left<l-2||r.right>right+2||r.top<t-2||r.bottom>b+2||!hit||!(hit===e||e.contains(hit)||hit.contains(e)))failures.push({control:e.dataset.testid||e.id||e.className||e.tagName,box:[r.x,r.y,r.width,r.height].map(Math.round)});
 }
 const empty=[];
 for(const e of scope.querySelectorAll('.works .list-pane .empty,.empty-state')){
  if(!visible(e))continue;
  // Measure against the visible card edge, including its padding. Some pages
  // use an unbordered inner empty-state wrapper inside an already padded card.
  let pane=e;
  for(let a=e;a&&a!==root&&a!==document.body;a=a.parentElement){const s=getComputedStyle(a);if(parseFloat(s.borderLeftWidth)>0&&parseFloat(s.borderRightWidth)>0){pane=a;break;}}
  const p=pane.getBoundingClientRect(),range=document.createRange();range.selectNodeContents(e);const text=range.getBoundingClientRect(),scale=p.width/pane.offsetWidth;empty.push({left:(text.left-p.left)/scale,right:(p.right-text.right)/scale});
 }
 return {controls:controls.length,failures,empty,horizontal:root.scrollWidth>root.clientWidth+2||document.documentElement.scrollWidth>innerWidth+2};
}
