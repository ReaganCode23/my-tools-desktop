import {cp,mkdir,rm,readFile,writeFile} from 'node:fs/promises';
await rm('dist',{recursive:true,force:true});await mkdir('dist',{recursive:true});await cp('app','dist',{recursive:true});
for(const file of ['app/little-sign/style.css','dist/little-sign/style.css']){let css=await readFile(file,'utf8');css=css.replace(/^@import[^;]+;\s*/,'').replaceAll("'DM Sans'","system-ui").replaceAll("'Instrument Serif'","Georgia");await writeFile(file,css);}
// Production app relies on Tauri's injected CSP, including its per-build script hashes.
const page=await readFile('dist/index.html','utf8');await writeFile('dist/index.html',page.replace(/<meta http-equiv="Content-Security-Policy"[^>]+>/,''));
console.log('Prepared offline app assets.');
