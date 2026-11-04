export function parsePages(input,total){
 if(!Number.isInteger(total)||total<1)throw Error('This PDF has no pages.');
 const value=input.trim();if(!value)throw Error('Enter a page range, such as 1-3, 5.');
 const pages=[],seen=new Set();
 for(const part of value.split(',')){
  const match=part.trim().match(/^(\d+)(?:\s*-\s*(\d+))?$/);if(!match)throw Error('Use page numbers or ranges, like 1-3, 5.');
  const start=Number(match[1]),end=Number(match[2]||match[1]);if(start<1||end<start||end>total)throw Error(`Choose pages from 1 to ${total}, with ranges in ascending order.`);
  for(let n=start;n<=end;n++)if(!seen.has(n)){seen.add(n);pages.push(n-1);}
 }
 return pages;
}
export function randomInt(max,cryptoSource=globalThis.crypto){
 if(!Number.isSafeInteger(max)||max<1||max>0x100000000)throw Error('Invalid random range.');
 const limit=Math.floor(0x100000000/max)*max,buffer=new Uint32Array(1);let n;
 do{cryptoSource.getRandomValues(buffer);n=buffer[0];}while(n>=limit);return n%max;
}
export function makePassword(length,groups,cryptoSource=globalThis.crypto){
 if(!Number.isInteger(length)||length<8||length>128)throw Error('Choose a length between 8 and 128.');
 if(!groups.length||groups.some(g=>!g.length))throw Error('Choose at least one character type.');
 const all=groups.join(''),chars=groups.map(g=>g[randomInt(g.length,cryptoSource)]);
 while(chars.length<length)chars.push(all[randomInt(all.length,cryptoSource)]);
 for(let i=chars.length-1;i>0;i--){const j=randomInt(i+1,cryptoSource);[chars[i],chars[j]]=[chars[j],chars[i]];}
 return chars.join('');
}
export function safeFileName(name,suffix,ext){return name.replace(/\.[^.]+$/,'').replace(/[<>:"/\\|?*\x00-\x1f]/g,'_').slice(0,160)+suffix+'.'+ext;}
