(function(){
const $=s=>document.querySelector(s), $$=s=>document.querySelectorAll(s), page=$('#page'), sc=$('#scroller'), wrap=$('#wrap');
// ---------- seed prose (fake chapter, long enough that windows start off-screen)
const title="From the desk of the harbourmaster";
const paras=[
"The lamps along the quay were lit one by one, and Mara counted them the way other people count steps, as a promise that the walk would end.",
"She had not meant to come back. The letter in her coat pocket said so in her own handwriting, though the ink had run where the rain found it.",
"Behind her the tide turned over its heavy shoulders and began, quietly, to take the harbour back.",
"The fish market had closed hours ago. Its awnings were rolled and tied, and the gutters still ran with the grey water the porters sluiced down at the end of every day, carrying scales and ice and the occasional bright eye of a mackerel out toward the sea.",
"Mara walked past the chandler's, past the rope works, past the low door of the tavern where her father had once lost a boat on a hand of cards and won it back on the next. The windows were fogged. Someone inside was singing badly and with great conviction.",
"At the end of the quay the harbourmaster's office stood on its stilts like a heron, one lamp burning in the upper window. She had known it would be lit. It was always lit.",
"She stopped at the foot of the steps. The wood was slick and green at the edges, and the handrail had been replaced since she left; the new one was too smooth, too pale, a stranger's hand where an old friend's should be.",
"Twelve years. She tried the number aloud and it sounded smaller than it was, the way the harbour itself had shrunk, the way the hills behind the town had lowered their shoulders while she was gone.",
"A gull landed on the rail beside her and regarded her with one cold eye, as if it, too, remembered what she had taken when she went.",
"The door at the top of the steps opened before she reached it. Light fell down the stairs in a long yellow tongue, and in the middle of it stood the harbourmaster, older, heavier, holding a mug of something that steamed.",
"\"You'll want to come in out of that,\" he said, as though she had been gone an afternoon. \"The kettle's only just boiled.\"",
"Mara did not move. The letter was a stone in her pocket. She had rehearsed this on the train, on the coach, on the long walk down from the station, and none of the rehearsals had begun with tea.",
"\"I didn't write,\" she said.",
"\"No,\" the harbourmaster agreed. \"But the sea did. It's been bringing your name in on every tide since spring.\"",
"She climbed the steps. Inside, the office was exactly as she remembered it and entirely different: the same charts pinned to the same walls, the same brass clock with its stubborn second hand, but a new chair by the stove, and on the desk a photograph she did not recognise, turned face down.",
"He poured her tea without asking how she took it and got it right, and that, more than anything, nearly undid her.",
"Outside, the last of the boats came in under the breakwater, its lamp swinging, its engine coughing like an old man on a cold morning. Mara watched it through the salt-blurred glass and waited for him to ask why she had come.",
"He didn't ask. He sat in the new chair, stretched his feet toward the stove, and began to talk about the price of diesel.",
"Later she would remember that as the moment she understood how much had changed, and how little."
];
const loreDB={
  mara:{name:"Mara Vell",alias:"Mara",kind:"character entry",use:"include",book:"1",ch:"1",note:""},
  orrin:{name:"Orrin Tade",alias:"the harbourmaster",kind:"character entry",type:"Character",use:"include",book:"1",ch:"1",note:""},
  harbour:{name:"Greywater Harbour",alias:"harbour",kind:"place entry",type:"Place",use:"hidden",book:"1",ch:"1",note:""},
  clock:{name:"The Brass Clock",alias:"brass clock",kind:"object entry",type:"Object",use:"include",book:"1",ch:"1",note:""}
};
loreDB.mara.type="Character";
const LEVEL={include:"Include",hidden:"Known, don't reveal",out:"Leave out"};
const esc=t=>t.replace(/&/g,'&amp;').replace(/</g,'&lt;');
function mark(t){return esc(t).replace(/\bMara\b/g,'<span class="lore" data-lore="mara">Mara</span>')
  .replace(/the harbourmaster(?!'s)/g,'<span class="lore" data-lore="orrin">the harbourmaster</span>')
  .replace(/\bharbour\b/g,'<span class="lore" data-lore="harbour">harbour</span>')
  .replace(/brass clock/g,'<span class="lore" data-lore="clock">brass clock</span>');}
page.innerHTML='<h1>'+title+'</h1>'+paras.map(t=>'<p>'+mark(t)+'</p>').join('');
const CARET_PARA=16;              // caret starts at the end of this paragraph (0-based)
const WIN={continue:6,resume:14}; // fake: window = N paragraphs before the caret paragraph
const canned="A boy ran past with a tin bucket, scattering silver sprats across the stones, and nobody turned to look. Mara pressed her palm flat against the cold iron of the railing and let the old rhythm settle in her, the creak of the moorings, the far bell, the salt-sour breath of the water. Whatever she had left unsaid here would keep a little longer.";
const st={recenter:'typing',timer:null,streaming:null,show:{continue:true,resume:true}};

// ---------- caret + recenter (typing only, ~80px dead zone)
const DEAD_ZONE=80;
function caretRect(){const s=getSelection();if(!s.rangeCount||!page.contains(s.anchorNode))return null;
  const r=s.getRangeAt(0).cloneRange();r.collapse(true);let b=r.getClientRects()[0];
  if(!b){const n=s.anchorNode,e=n.nodeType==1?n:n.parentElement;b=e.getBoundingClientRect();}return b;}
function recenter(){const b=caretRect();if(!b)return;const mid=b.top+b.height/2-innerHeight/2;
  if(Math.abs(mid)>DEAD_ZONE)sc.scrollTo({top:sc.scrollTop+mid,behavior:'smooth'});}
page.addEventListener('input',()=>{ if(st.recenter==='typing') requestAnimationFrame(recenter); markSaving(); schedule();});

// ---------- context-window brackets + off-screen "window starts" flags (fake ranges)
function caretPara(){const s=getSelection();let n=s.rangeCount&&page.contains(s.anchorNode)?s.anchorNode:null;
  while(n&&n.parentNode!==page)n=n.parentNode;return n&&n.tagName==='P'?n:page.lastElementChild;}
let lastCaretPara=null;
function windowStart(kind){const ps=[...page.querySelectorAll('p')],i=Math.max(0,ps.indexOf(lastCaretPara||caretPara()));
  return ps[Math.max(0,i-WIN[kind])];}
let raf=0;function schedule(){if(!raf)raf=requestAnimationFrame(()=>{raf=0;layout();});}
function layout(){
  const wr=wrap.getBoundingClientRect(), cb=caretRect();
  if(cb)lastCaretPara=caretPara();
  const endY=(cb?cb.bottom:(lastCaretPara||page.lastElementChild).getBoundingClientRect().bottom)-wr.top;
  const flags=$('#flags');flags.innerHTML='';
  const inwin=new Set();let lastY=-99;
  ['resume','continue'].forEach(kind=>{
    const br=$('#br-'+kind),on=st.show[kind];br.hidden=!on;if(!on)return;
    const start=windowStart(kind),sr=start.getBoundingClientRect(),top=sr.top-wr.top;
    br.style.top=top+'px';br.style.height=Math.max(12,endY-top)+'px';
    let n=start;while(n){inwin.add(n);if(n===lastCaretPara)break;n=n.nextElementSibling;}
    // gutter label attached to its bracket: sits at the bracket's top, or at the viewport top when the start is off-screen
    const off=sr.top<60,bTop=wr.top+top,bBot=wr.top+top+Math.max(12,endY-top);
    let y=Math.min(Math.max(bTop,64),bBot-34);if(kind==='continue'&&y<lastY+38)y=lastY+38;lastY=y;
    const lineW=kind==='continue'?22:8,g=document.createElement('div');g.className='glab '+(kind==='continue'?'blue':'lav')+(off?' off':'');
    const b=document.createElement('button');b.className='flag '+(kind==='continue'?'blue':'lav');b.onmousedown=e=>e.preventDefault();
    let t=kind==='continue'?'Continue':'Resume',sub='';
    if(off){const d=-sr.top,pages=d/innerHeight;sub='starts &uarr; '+(pages<.6?'just above':(Math.round(pages*2)/2)+(pages<1.25?' page':' pages')+' up');
      b.title='Scroll to where the '+kind+' window starts';b.onclick=()=>sc.scrollTo({top:sc.scrollTop+start.getBoundingClientRect().top-120,behavior:'smooth'});}
    b.innerHTML='<b>'+t+' window</b>'+(sub?'<small>'+sub+'</small>':'');
    const ln=document.createElement('i');ln.style.width=lineW+'px';g.append(b,ln);
    g.style.top=y+'px';g.style.right=(innerWidth-wr.left+46-lineW)+'px';flags.appendChild(g);
  });
  $$('.lore').forEach(l=>{const p=l.closest('p'),w=inwin.has(p)&&!l.dataset.ignored;l.classList.toggle('inwin',w);
    const d=loreDB[l.dataset.lore];if(d)l.dataset.use=d.use;});
  renderLoreWin();
}
sc.addEventListener('scroll',schedule,{passive:true});addEventListener('resize',schedule);
document.addEventListener('selectionchange',()=>{if(page.contains(getSelection().anchorNode))schedule();});
['continue','resume'].forEach(k=>$('#set-w-'+k).onchange=e=>{st.show[k]=e.target.checked;schedule();});
function renderLoreWin(){
  const rows=Object.keys(loreDB).map(k=>({k,d:loreDB[k],els:[...$$('.lore.inwin[data-lore='+k+']')]})).filter(r=>r.els.length);
  const sig=rows.map(r=>r.k+r.els.length+r.d.use).join('|'),ul=$('#lorewin');if(ul.dataset.sig===sig)return;ul.dataset.sig=sig;
  $('#lw-count').textContent=rows.length?rows.length+' names':'';
  ul.innerHTML=rows.length?'':'<li class="mono">No lore names in the current window.</li>';
  rows.forEach(r=>{const li=document.createElement('li'),b=document.createElement('button');
    b.innerHTML='<span class="nm">'+esc(r.d.name)+'</span><span class="ct">'+r.els.length+(r.els.length>1?' matches':' match')+'</span><span class="tp">'+r.d.type+'</span><span class="lv '+r.d.use+'">'+LEVEL[r.d.use]+'</span>';
    b.title='Scroll to first match and open its lore card';b.onmousedown=e=>e.preventDefault();
    b.onclick=()=>{const el=r.els[0];saveSel();sc.scrollTo({top:sc.scrollTop+el.getBoundingClientRect().top-innerHeight/2});layout();showLore(el);};
    li.appendChild(b);ul.appendChild(li);});}

// ---------- save status
let saveT; const sb=$('#savebar'),stx=$('#savetext'),ssub=$('#savesub');
function setSave(){ if($('#set-fail').checked){sb.className='savebar fail';stx.textContent='Save stopped';ssub.textContent='· latest changes are not saved, retrying';}
  else {sb.className='savebar';stx.textContent=stx.dataset.s||'Saved';ssub.textContent='';}}
function markSaving(){ if($('#set-fail').checked)return; stx.dataset.s='Saving…';stx.textContent='Saving…';clearTimeout(saveT);saveT=setTimeout(()=>{stx.dataset.s='Saved';setSave()},900);}
function flash(msg){if($('#set-fail').checked)return;stx.textContent=msg;setTimeout(()=>{stx.textContent=stx.dataset.s||'Saved'},1500);}
$('#set-fail').onchange=setSave;

// ---------- settings
$('#set-recenter').onchange=e=>st.recenter=e.target.value;
$('#set-effect').onchange=e=>document.body.dataset.effect=e.target.value;
$('#btn-settings').onclick=e=>{e.stopPropagation();$('#pop').hidden=!$('#pop').hidden};

// ---------- panels (click/keyboard only)
let savedRange=null;
function saveSel(){const s=getSelection();savedRange=s.rangeCount&&page.contains(s.anchorNode)?s.getRangeAt(0).cloneRange():null;}
function restoreSel(){page.focus();if(savedRange){const s=getSelection();s.removeAllRanges();s.addRange(savedRange);}}
function closeAll(){$$('.panel.open').forEach(p=>p.classList.remove('open'));$$('.rail .on').forEach(b=>b!==$('#btn-continue')&&b.classList.remove('on'));
  $('#pop').hidden=true;$('#editbox').hidden=true;hideLore();}
function toggle(name){const p=$('#panel-'+name),was=p.classList.contains('open');closeAll();if(!was){p.classList.add('open');const b=document.querySelector('[data-panel='+name+']');b&&b.classList.add('on');}}
$$('[data-panel]').forEach(b=>b.onclick=e=>{e.stopPropagation();toggle(b.dataset.panel)});
addEventListener('keydown',e=>{
  if(e.key==='Escape'){const back=!$('#lore').hidden||!$('#editbox').hidden;closeAll();back?restoreSel():page.focus();return;}
  if(e.altKey&&e.code==='KeyL'){e.preventDefault();openLoreByKey();return;}
  if(!$('#editbox').hidden&&e.target===$('#edit-instr')&&e.key==='Enter'&&!e.shiftKey){e.preventDefault();applyEdit();return;}
  if((e.metaKey||e.ctrlKey)&&e.key==='['){e.preventDefault();toggle('nav');}
  else if((e.metaKey||e.ctrlKey)&&e.key===']'){e.preventDefault();toggle('ctx');}
  else if(e.key==='?'&&e.altKey){e.preventDefault();$('#switcher').hidden=!$('#switcher').hidden;}
  else if(e.key==='?'&&(document.activeElement!==page||e.ctrlKey)&&!/TEXTAREA|INPUT|SELECT/.test(document.activeElement.tagName)){$('#switcher').hidden=!$('#switcher').hidden;}
});

// ---------- fake streaming: inserts a paragraph after caret paragraph; caret untouched
function settle(nodes){setTimeout(()=>nodes.forEach(x=>x.classList.add('old')),250);} // tint fades via CSS transition
function stop(){if(!st.streaming)return;clearInterval(st.timer);st.streaming=null;$('#btn-continue').classList.remove('on');settle([...$$('.ai:not(.old)')]);}
function startStream(){ if(st.streaming){stop();return;}
  const after=caretPara(),p=document.createElement('p');p.className='aip';after.after(p);st.streaming=p;let i=0;
  $('#btn-continue').classList.add('on');
  st.timer=setInterval(()=>{const chunk=canned.slice(i,i+3);i+=3; // 10 ticks/s x 3 chars = 30 chars/s
    const sp=document.createElement('span');sp.className='ai';sp.textContent=chunk;p.appendChild(sp);
    if(i>=canned.length)stop();},100);}
$('#btn-continue').onclick=e=>{e.stopPropagation();startStream();};

// ---------- Edit selection: pink post-it instruction note
$('#btn-edit').onclick=e=>{e.stopPropagation();const s=getSelection();
  if(s.isCollapsed||!page.contains(s.anchorNode)){flash('Select text to edit');return;}
  closeAll();saveSel();$('#editbox').hidden=false;$('#btn-edit').classList.add('on');$('#edit-instr').focus();};
function applyEdit(){if(!savedRange)return;restoreSel();const r=savedRange,t=r.toString();
  const sp=document.createElement('span');sp.className='ai';sp.textContent=t.charAt(0).toUpperCase()+t.slice(1); // fake rewrite
  r.deleteContents();r.insertNode(sp);savedRange=null;$('#editbox').hidden=true;$('#btn-edit').classList.remove('on');
  $('#edit-instr').value='';settle([sp]);markSaving();}
$('#edit-apply').onclick=applyEdit;

// ---------- lore card: 600ms hover delay, or Alt+L for the nearest in-window name
const card=$('#lore');let loreT=0,hideT=0,cur=null;
function showLore(span){
  clearTimeout(hideT);if(cur)cur.classList.remove('active');cur=span;span.classList.add('active');
  const d=loreDB[span.dataset.lore];
  $('#lore-name').textContent=d.name;
  $('#lore-why').innerHTML='Matched alias <b>&ldquo;'+esc(d.alias)+'&rdquo;</b> &middot; '+d.kind+' &middot; inside the current window';
  card.querySelector('input[value='+d.use+']').checked=true;
  $('#lore-book').value=d.book;$('#lore-ch').value=d.ch;$('#lore-note').value=d.note;
  card.hidden=false;const r=span.getBoundingClientRect(),h=card.offsetHeight;
  let top=r.bottom+8;if(top+h>innerHeight-12)top=r.top-h-8;
  card.style.top=Math.max(12,top)+'px';card.style.left=Math.min(innerWidth-card.offsetWidth-70,Math.max(70,r.left-20))+'px';}
function hideLore(){clearTimeout(loreT);card.hidden=true;if(cur)cur.classList.remove('active');cur=null;}
page.addEventListener('mouseover',e=>{const l=e.target.closest('.lore.inwin');if(!l)return;
  clearTimeout(hideT);if(cur===l)return;clearTimeout(loreT);loreT=setTimeout(()=>showLore(l),600);});
page.addEventListener('mouseout',e=>{const l=e.target.closest('.lore.inwin');if(!l)return;
  clearTimeout(loreT);hideT=setTimeout(()=>{if(!card.matches(':hover')&&!card.contains(document.activeElement))hideLore();},350);});
card.addEventListener('mouseleave',()=>{hideT=setTimeout(()=>{if(!card.contains(document.activeElement))hideLore();},350);});
card.addEventListener('mouseenter',()=>clearTimeout(hideT));
function openLoreByKey(){layout();const ls=[...$$('.lore.inwin')];if(!ls.length){flash('No lore names in the current window');return;}
  const cb=caretRect(),y=cb?cb.top:innerHeight/2;
  ls.sort((a,b)=>Math.abs(a.getBoundingClientRect().top-y)-Math.abs(b.getBoundingClientRect().top-y));
  saveSel();showLore(ls[0]);card.querySelector('input:checked').focus();}
function curData(){return cur&&loreDB[cur.dataset.lore];}
card.querySelectorAll('input[name=lore-use]').forEach(i=>i.onchange=()=>{const d=curData();if(d){d.use=i.value;schedule();}});
$('#lore-book').onchange=e=>{const d=curData();if(d)d.book=e.target.value;};
$('#lore-ch').onchange=e=>{const d=curData();if(d)d.ch=e.target.value;};
$('#lore-note').oninput=e=>{const d=curData();if(d)d.note=e.target.value;};
$('#lore-ignore').onclick=()=>{if(!cur)return;cur.dataset.ignored='1';hideLore();restoreSel();schedule();};

// mousedown on rail buttons must not steal caret/selection
$$('.rail button').forEach(b=>b.addEventListener('mousedown',e=>e.preventDefault()));
sc.addEventListener('mousedown',e=>{if(!e.target.closest('.pop')){$('#pop').hidden=true}});

// ---------- state switcher
function focusEnd(){const p=page.querySelectorAll('p')[CARET_PARA];const r=document.createRange();r.selectNodeContents(p);r.collapse(false);const s=getSelection();s.removeAllRanges();s.addRange(r);page.focus();
  const b=caretRect();b&&sc.scrollTo({top:sc.scrollTop+b.top+b.height/2-innerHeight/2});schedule();}
$$('[data-state]').forEach(b=>b.onclick=()=>{
  stop();closeAll();$$('p.aip').forEach(p=>p.remove());$('#set-fail').checked=false;setSave();
  focusEnd();
  if(b.dataset.state==='stream')startStream();
  if(b.dataset.state==='panel')toggle('ctx');
  if(b.dataset.state==='lore')setTimeout(openLoreByKey,60);
  if(b.dataset.state==='fail'){$('#set-fail').checked=true;setSave();}});
setTimeout(focusEnd,50);
})();
