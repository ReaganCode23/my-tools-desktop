import { buildSignedPdf } from './pdf-export.mjs';
const $ = id => document.getElementById(id);
const state = {bytes:null,doc:null,fileName:'',page:1,views:new Map(),placements:[],selected:null,signature:null,placing:false,busy:false};
let pdfjs, nextId=1, strokes=[], activeStroke=null, ink='#202a39', toastTimer;
const ready = import('./vendor/pdf.mjs').then(m=>{pdfjs=m;pdfjs.GlobalWorkerOptions.workerSrc=new URL('./vendor/pdf.worker.mjs',import.meta.url).href;return m;});
ready.catch(()=>notify('The PDF tools could not load. Please refresh and try again.'));
function notify(message){$('status').textContent=message;$('status').hidden=false;clearTimeout(toastTimer);toastTimer=setTimeout(()=>$('status').hidden=true,6000);}
function update(){
  $('download').disabled=state.busy||!state.placements.length;
  $('prev').disabled=state.busy||state.page<=1;$('next').disabled=state.busy||!state.doc||state.page>=state.doc.numPages;
  $('add-signature').disabled=state.busy;$('replace').disabled=state.busy;
  $('signature-count').textContent=state.placements.length?`${state.placements.length} item${state.placements.length===1?'':'s'} placed`:'No items placed yet';
  const p=state.placements.find(p=>p.id===state.selected);
  $('selected-tools').hidden=!p;if(p){$('signature-size').value=Math.round(p.w*100);$('size-value').textContent=Math.round(p.w*100)+'%';$('selection-label').textContent=p.type==='text'?'SELECTED TEXT BOX':'SELECTED SIGNATURE';$('remove').textContent=p.type==='text'?'Remove text box':'Remove signature';$('text-tools').hidden=p.type!=='text';if(p.type==='text'){if(document.activeElement!==$('text-content'))$('text-content').value=p.text;$('text-font-size').value=p.fontSize;$('font-size-value').textContent=p.fontSize+' pt';}}
  $('add-text').disabled=state.busy;$('add-date').disabled=state.busy;
  $('placements').classList.toggle('placing',state.placing);
  $('page-tip').textContent=state.placing?`Click anywhere on the page to place your ${state.pendingText?'text box':'signature'}.`:'Add a signature, printed name, or date. Drag any item to position it.';
  $('placements').dataset.placementLabel=state.pendingText?'Click to place your text box':'Click to place your signature';
}
async function loadPDF(bytes,name){
  if(state.busy)return;state.busy=true;update();notify('Opening your PDF…');
  let nextDoc;
  try{
    await ready;if(!window.PDFLib)throw Error('The PDF editor could not load. Please refresh.');
    await PDFLib.PDFDocument.load(bytes.slice());
    nextDoc=await pdfjs.getDocument({data:bytes.slice(),cMapUrl:new URL('./vendor/cmaps/',import.meta.url).href,cMapPacked:true,standardFontDataUrl:new URL('./vendor/standard_fonts/',import.meta.url).href,isEvalSupported:false}).promise;
    const oldDoc=state.doc;
    Object.assign(state,{bytes:bytes.slice(),doc:nextDoc,fileName:name,page:1,views:new Map(),placements:[],selected:null,placing:false,pendingText:null});
    $('welcome').hidden=true;$('editor').hidden=false;$('filename').textContent=name;
    $('filemeta').textContent=`${nextDoc.numPages} page${nextDoc.numPages===1?'':'s'} · ${(bytes.length/1024/1024).toFixed(1)} MB`;
    await renderPage();if(oldDoc)await oldDoc.destroy();
    $('status').hidden=true;openSignature();
  }catch(error){
    const encrypted=/encrypt|password/i.test(error.message||'');notify(encrypted?'This PDF is password-protected. Please upload an unlocked copy.':'Could not open this PDF. Try a valid, unlocked PDF file.');
    if(nextDoc&&nextDoc!==state.doc)await nextDoc.destroy();
  }finally{state.busy=false;update();$('file-input').value='';}
}
async function readFile(file){
  if(!file)return;
  if(file.size>50*1024*1024){notify('Choose a PDF smaller than 50 MB.');return;}
  if(!/\.pdf$/i.test(file.name)&&file.type!=='application/pdf'){notify('Please choose a PDF file.');return;}
  try{await loadPDF(new Uint8Array(await file.arrayBuffer()),file.name);}catch{notify('This file could not be read. Please try again.');}
}
$('upload').onclick=()=>{if(!state.busy)$('file-input').click();};
$('replace').onclick=()=>$('file-input').click();
$('file-input').onchange=e=>readFile(e.target.files[0]);
const drop=$('dropzone');
drop.addEventListener('dragover',e=>{e.preventDefault();drop.classList.add('dragover');});
drop.addEventListener('dragleave',()=>drop.classList.remove('dragover'));
drop.addEventListener('drop',e=>{e.preventDefault();drop.classList.remove('dragover');readFile(e.dataTransfer.files[0]);});
drop.addEventListener('click',e=>{if(!e.target.closest('button')&&!state.busy)$('file-input').click();});
window.addEventListener('dragover',e=>e.preventDefault());window.addEventListener('drop',e=>e.preventDefault());
async function renderPage(){
  const p=await state.doc.getPage(state.page),view=p.getViewport({scale:1});state.views.set(state.page,view);
  const width=Math.min(850,Math.max(200,$('page-wrap').clientWidth-56));
  const scale=width/view.width,device=Math.min(window.devicePixelRatio||1,2);
  const v=p.getViewport({scale:scale*device}),canvas=$('pdf-canvas');canvas.width=Math.ceil(v.width);canvas.height=Math.ceil(v.height);
  $('page').style.maxWidth=width+'px';$('page').style.aspectRatio=`${view.width} / ${view.height}`;
  await p.render({canvasContext:canvas.getContext('2d'),viewport:v}).promise;
  $('page-label').textContent=`Page ${state.page} of ${state.doc.numPages}`;
  renderPlacements();update();
}
async function navigate(delta){if(state.busy)return;state.busy=true;state.selected=null;update();try{state.page+=delta;await renderPage();$('page-wrap').scrollTop=0;}catch{notify('This page could not be displayed.');}finally{state.busy=false;update();}}
$('prev').onclick=()=>navigate(-1);$('next').onclick=()=>navigate(1);
function openSignature(){if(!state.doc)return;strokes=[];activeStroke=null;$('signature-dialog').showModal();drawPad();$('use-signature').disabled=true;}
function closeSignature(){$('signature-dialog').close();activeStroke=null;}
$('signature-preview').onclick=openSignature;$('close-dialog').onclick=closeSignature;$('cancel-dialog').onclick=closeSignature;
$('add-signature').onclick=()=>{state.pendingText=null;if(!state.signature)openSignature();else{state.placing=true;state.selected=null;renderPlacements();update();}};
function addText(text){state.pendingText={text,fontSize:14,type:'text'};state.placing=true;state.selected=null;renderPlacements();update();}
$('add-text').onclick=()=>addText('Printed name');
$('add-date').onclick=()=>addText(new Intl.DateTimeFormat(undefined,{year:'numeric',month:'2-digit',day:'2-digit'}).format(new Date()));
function updateTextGraphic(p){
  const view=state.views.get(p.page),width=p.w*view.width,canvas=document.createElement('canvas'),ctx=canvas.getContext('2d');
  ctx.font=`${p.fontSize}px Arial, sans-serif`;
  const lines=[];
  for(const paragraph of p.text.split('\n')){let line='';for(const ch of paragraph){if(ctx.measureText(line+ch).width>width-4&&line){lines.push(line);line=ch;}else line+=ch;}lines.push(line);}
  const height=Math.min(view.height,Math.max(p.fontSize*1.4,lines.length*p.fontSize*1.4));
  p.h=height/view.height;p.y=clamp(p.y||0,0,1-p.h);p.ratio=width/height;
  const density=Math.min(3,4096/Math.max(width,height));canvas.width=Math.ceil(width*density);canvas.height=Math.ceil(height*density);ctx.scale(density,density);ctx.font=`${p.fontSize}px Arial, sans-serif`;ctx.fillStyle='#202a39';ctx.textBaseline='top';
  lines.forEach((line,i)=>ctx.fillText(line,1,i*p.fontSize*1.4+1));p.url=canvas.toDataURL('image/png');
}
$('text-content').maxLength=2000;
$('text-content').oninput=e=>{const p=state.placements.find(p=>p.id===state.selected);if(p?.type!=='text'||state.busy)return;p.text=e.target.value;updateTextGraphic(p);renderPlacements();update();};
$('text-font-size').oninput=e=>{const p=state.placements.find(p=>p.id===state.selected);if(p?.type!=='text'||state.busy)return;p.fontSize=Number(e.target.value);updateTextGraphic(p);renderPlacements();update();};
const pad=$('signature-pad');
function drawPad(){
  const rect=pad.getBoundingClientRect(),dpr=window.devicePixelRatio||1;
  pad.width=Math.max(1,Math.round(rect.width*dpr));pad.height=Math.max(1,Math.round(rect.height*dpr));
  const ctx=pad.getContext('2d');ctx.scale(dpr,dpr);ctx.lineCap='round';ctx.lineJoin='round';ctx.lineWidth=2.5;
  for(const stroke of strokes){ctx.strokeStyle=stroke.color;ctx.fillStyle=stroke.color;
    if(stroke.points.length===1){ctx.beginPath();ctx.arc(stroke.points[0].x*rect.width,stroke.points[0].y*rect.height,1.25,0,Math.PI*2);ctx.fill();continue;}
    ctx.beginPath();stroke.points.forEach((p,i)=>{if(!i)ctx.moveTo(p.x*rect.width,p.y*rect.height);else ctx.lineTo(p.x*rect.width,p.y*rect.height);});ctx.stroke();}
  $('use-signature').disabled=!strokes.length;
}
function point(e){const r=pad.getBoundingClientRect();return{x:Math.max(0,Math.min(1,(e.clientX-r.left)/r.width)),y:Math.max(0,Math.min(1,(e.clientY-r.top)/r.height))};}
pad.addEventListener('pointerdown',e=>{if(e.button!==0)return;e.preventDefault();pad.setPointerCapture(e.pointerId);activeStroke={color:ink,points:[point(e)]};strokes.push(activeStroke);drawPad();});
pad.addEventListener('pointermove',e=>{if(!activeStroke)return;e.preventDefault();for(const event of e.getCoalescedEvents?.()||[e])activeStroke.points.push(point(event));drawPad();});
function endStroke(){activeStroke=null;}pad.addEventListener('pointerup',endStroke);pad.addEventListener('pointercancel',endStroke);pad.addEventListener('lostpointercapture',endStroke);
$('clear').onclick=()=>{strokes=[];activeStroke=null;drawPad();};$('undo').onclick=()=>{strokes.pop();activeStroke=null;drawPad();};
document.querySelectorAll('.swatch').forEach(button=>button.onclick=()=>{ink=button.dataset.color;document.querySelectorAll('.swatch').forEach(b=>{b.classList.toggle('active',b===button);b.setAttribute('aria-pressed',String(b===button));});strokes.forEach(s=>s.color=ink);drawPad();});
$('use-signature').onclick=()=>{
  if(!strokes.length)return;
  const ctx=pad.getContext('2d'),pixels=ctx.getImageData(0,0,pad.width,pad.height).data;
  let left=pad.width,right=0,top=pad.height,bottom=0;
  for(let y=0;y<pad.height;y++)for(let x=0;x<pad.width;x++)if(pixels[(y*pad.width+x)*4+3]){left=Math.min(left,x);right=Math.max(right,x);top=Math.min(top,y);bottom=Math.max(bottom,y);}
  if(right<left)return;const margin=8;left=Math.max(0,left-margin);top=Math.max(0,top-margin);right=Math.min(pad.width-1,right+margin);bottom=Math.min(pad.height-1,bottom+margin);
  const cropped=document.createElement('canvas');cropped.width=right-left+1;cropped.height=bottom-top+1;cropped.getContext('2d').drawImage(pad,left,top,cropped.width,cropped.height,0,0,cropped.width,cropped.height);
  state.signature={url:cropped.toDataURL('image/png'),ratio:cropped.width/cropped.height};
  const image=document.createElement('img');image.src=state.signature.url;image.alt='Your drawn signature';$('signature-preview').replaceChildren(image);
  state.pendingText=null;state.placing=true;state.selected=null;closeSignature();renderPlacements();update();$('placements').focus();
};
function dimensions(w,ratio){const v=state.views.get(state.page);let h=w*v.width/v.height/ratio;if(h>.85){w*=.85/h;h=.85;}return{w,h};}
const clamp=(value,min,max)=>Math.max(min,Math.min(max,value));
$('placements').addEventListener('pointerdown',e=>{
  if(!state.placing||(!state.signature&&!state.pendingText)||state.busy||e.target.closest('.placed'))return;
  const rect=$('placements').getBoundingClientRect();let p;
  if(state.pendingText){p={id:nextId++,page:state.page,...state.pendingText,w:.30,y:0};updateTextGraphic(p);p.x=clamp((e.clientX-rect.left)/rect.width,0,1-p.w);p.y=clamp((e.clientY-rect.top)/rect.height-p.h/2,0,1-p.h);state.pendingText=null;}
  else{const d=dimensions(.28,state.signature.ratio);p={id:nextId++,type:'signature',page:state.page,url:state.signature.url,ratio:state.signature.ratio,...d,x:clamp((e.clientX-rect.left)/rect.width-d.w/2,0,1-d.w),y:clamp((e.clientY-rect.top)/rect.height-d.h/2,0,1-d.h)};}
  state.placements.push(p);state.selected=p.id;state.placing=false;renderPlacements();update();
});
function renderPlacements(){
  $('placements').replaceChildren();
  for(const p of state.placements.filter(p=>p.page===state.page)){
    const box=document.createElement('div');box.className='placed'+(p.id===state.selected?' selected':'');box.tabIndex=0;box.setAttribute('role','group');box.setAttribute('aria-label',`Placed ${p.type==='text'?'text box':'signature'}. Use arrow keys to move. Delete to remove.`);
    const image=document.createElement('img');image.src=p.url;image.alt=p.type==='text'?p.text:'Signature';box.append(image);
    const handle=document.createElement('span');handle.className='resize-handle';box.append(handle);
    function position(){Object.assign(box.style,{left:p.x*100+'%',top:p.y*100+'%',width:p.w*100+'%',height:p.h*100+'%'});}position();
    box.addEventListener('pointerdown',e=>{
      if(e.button!==0||state.busy)return;e.stopPropagation();e.preventDefault();state.placing=false;state.selected=p.id;
      document.querySelectorAll('.placed').forEach(el=>el.classList.remove('selected'));box.classList.add('selected');box.focus();update();box.setPointerCapture(e.pointerId);
      const resize=e.target===handle,r=$('placements').getBoundingClientRect(),start={x:e.clientX,y:e.clientY,px:p.x,py:p.y,w:p.w,h:p.h};
      const move=event=>{const dx=(event.clientX-start.x)/r.width,dy=(event.clientY-start.y)/r.height;
        if(resize){if(p.type==='text'){p.w=clamp(start.w+dx,.05,1-p.x);updateTextGraphic(p);image.src=p.url;}else{const v=state.views.get(p.page),factor=v.width/v.height/p.ratio;const w=clamp(start.w+dx,.03,Math.min(1-p.x,(1-p.y)/factor));p.w=w;p.h=w*factor;}}
        else{p.x=clamp(start.px+dx,0,1-p.w);p.y=clamp(start.py+dy,0,1-p.h);}position();update();};
      const end=()=>{box.removeEventListener('pointermove',move);box.removeEventListener('pointerup',end);box.removeEventListener('pointercancel',end);box.removeEventListener('lostpointercapture',end);};
      box.addEventListener('pointermove',move);box.addEventListener('pointerup',end);box.addEventListener('pointercancel',end);box.addEventListener('lostpointercapture',end);
    });
    box.onfocus=()=>{state.selected=p.id;document.querySelectorAll('.placed').forEach(el=>el.classList.toggle('selected',el===box));update();};
    box.onkeydown=e=>{if(state.busy)return;const delta=e.shiftKey?.02:.003;const actions={ArrowLeft:()=>p.x=clamp(p.x-delta,0,1-p.w),ArrowRight:()=>p.x=clamp(p.x+delta,0,1-p.w),ArrowUp:()=>p.y=clamp(p.y-delta,0,1-p.h),ArrowDown:()=>p.y=clamp(p.y+delta,0,1-p.h)};if(actions[e.key]){e.preventDefault();actions[e.key]();position();}else if(e.key==='Delete'||e.key==='Backspace'){e.preventDefault();removeSelected();}};
    $('placements').append(box);
  }
}
function removeSelected(){state.placements=state.placements.filter(p=>p.id!==state.selected);state.selected=null;renderPlacements();update();}
$('remove').onclick=removeSelected;
$('signature-size').oninput=e=>{const p=state.placements.find(p=>p.id===state.selected);if(!p||state.busy)return;if(p.type==='text'){p.w=Number(e.target.value)/100;updateTextGraphic(p);}else Object.assign(p,dimensions(Number(e.target.value)/100,p.ratio));p.x=clamp(p.x,0,1-p.w);p.y=clamp(p.y,0,1-p.h);renderPlacements();update();};
$('download').onclick=async()=>{
  if(!state.placements.length||state.busy)return;state.busy=true;update();const button=$('download');button.textContent='Preparing your PDF…';
  try{
    const bytes=await buildSignedPdf(PDFLib,state.bytes,state.placements,state.views),blob=new Blob([bytes],{type:'application/pdf'}),name=state.fileName.replace(/\.pdf$/i,'')+'-signed.pdf';
    if(window.parent!==window && window.parent.desktop?.saveBlob){const saved=await window.parent.desktop.saveBlob(blob,name);if(saved)notify('Your signed PDF is saved.');}
    else{const url=URL.createObjectURL(blob),link=document.createElement('a');link.href=url;link.download=name;document.body.append(link);link.click();link.remove();setTimeout(()=>URL.revokeObjectURL(url),60000);notify('Your signed PDF is ready. Check your downloads.');}
  }catch(error){console.error(error);notify('The signed PDF could not be created. Please try another unlocked PDF.');}
  finally{state.busy=false;button.innerHTML='Download signed PDF <span>↓</span>';update();}
};
$('demo').onclick=async()=>{
  if(state.busy)return;
  try{await ready;if(!window.PDFLib)throw Error();const {PDFDocument,StandardFonts,rgb}=PDFLib,doc=await PDFDocument.create(),regular=await doc.embedFont(StandardFonts.Helvetica),serif=await doc.embedFont(StandardFonts.TimesRoman);
    for(let i=0;i<2;i++){const p=doc.addPage([612,792]);p.drawText('LITTLE SIGN / PRACTICE COPY',{x:54,y:730,size:9,font:regular,color:rgb(.4,.49,.36)});p.drawLine({start:{x:54,y:708},end:{x:558,y:708},color:rgb(.82,.85,.78),thickness:1});
      p.drawText(i?'One more little thing.':'A place for your signature.',{x:54,y:638,size:29,font:serif,color:rgb(.17,.24,.20)});
      const lines=i?['You can add your signature on any page.','Use the arrows above to move between pages.','When you are happy, download your signed PDF.']:['This is a sample document for trying out Little Sign.','Draw your signature in the box, then click the page to place it.','Move it around and resize it until it feels just right.'];
      lines.forEach((line,j)=>p.drawText(line,{x:54,y:594-j*23,size:11,font:regular,color:rgb(.4,.45,.38)}));
      p.drawText('A little space to make your mark',{x:54,y:290,size:10,font:regular,color:rgb(.48,.54,.43)});p.drawLine({start:{x:54,y:219},end:{x:310,y:219},color:rgb(.65,.71,.60),thickness:1});p.drawText('Your signature',{x:54,y:201,size:9,font:regular,color:rgb(.48,.54,.43)});p.drawText('Sample only - no agreement or commitment.',{x:54,y:60,size:9,font:regular,color:rgb(.55,.59,.51)});p.drawText(`${i+1} / 2`,{x:535,y:60,size:9,font:regular,color:rgb(.55,.59,.51)});
    }await loadPDF(await doc.save(),'little-sign-sample.pdf');
  }catch{notify('The sample could not load. Please refresh and try again.');}
};
let resizeTimer;window.addEventListener('resize',()=>{clearTimeout(resizeTimer);resizeTimer=setTimeout(async()=>{if($('signature-dialog').open)drawPad();if(state.doc&&!state.busy){state.busy=true;update();try{await renderPage();}finally{state.busy=false;update();}}},200);});
update();
