export function transformText(action,value){
 if(value.length>1000000)throw Error('Text input is limited to 1 million characters.');
 switch(action){
  case'json-format':return JSON.stringify(JSON.parse(value),null,2);
  case'json-minify':return JSON.stringify(JSON.parse(value));
  case'base64-encode':{const b=new TextEncoder().encode(value);let s='';for(let i=0;i<b.length;i+=32768)s+=String.fromCharCode(...b.subarray(i,i+32768));return btoa(s);}
  case'base64-decode':{const b=Uint8Array.from(atob(value.replace(/\s/g,'')),c=>c.charCodeAt(0));return new TextDecoder('utf-8',{fatal:true}).decode(b);}
  case'url-encode':return encodeURIComponent(value);
  case'url-decode':return decodeURIComponent(value);
  case'sort-lines':return value.split(/\r?\n/).sort((a,b)=>a.localeCompare(b)).join('\n');
  case'unique-lines':return [...new Set(value.split(/\r?\n/))].join('\n');
  default:throw Error('Choose a text operation.');
 }
}
export function compareChecksum(actual,expected){const e=expected.trim().replace(/\s/g,'').toLowerCase();if(!e)return'empty';if(!/^[a-f0-9]+$/.test(e)||e.length!==actual.length)return'invalid';return e===actual.toLowerCase()?'match':'mismatch';}
