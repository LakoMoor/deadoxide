import fs from "node:fs";
import zlib from "node:zlib";
function png(p){const b=fs.readFileSync(p);let o=8,w,h,ct,idat=[];while(o<b.length){const l=b.readUInt32BE(o),t=b.toString("ascii",o+4,o+8);if(t==="IHDR"){w=b.readUInt32BE(o+8);h=b.readUInt32BE(o+12);ct=b[o+17];}if(t==="IDAT")idat.push(b.subarray(o+8,o+8+l));o+=12+l;}
const ch=ct===6?4:3,raw=zlib.inflateSync(Buffer.concat(idat)),st=w*ch,px=Buffer.alloc(h*st);let q=0;
for(let y=0;y<h;y++){const f=raw[q++];for(let x=0;x<st;x++){const a=x>=ch?px[y*st+x-ch]:0,u=y?px[(y-1)*st+x]:0,c=(x>=ch&&y)?px[(y-1)*st+x-ch]:0;let v=raw[q++];
if(f===1)v+=a;else if(f===2)v+=u;else if(f===3)v+=(a+u)>>1;else if(f===4){const pp=a+u-c,pa=Math.abs(pp-a),pb=Math.abs(pp-u),pc=Math.abs(pp-c);v+=(pa<=pb&&pa<=pc)?a:(pb<=pc?u:c);}px[y*st+x]=v&255;}}
return {w,h,ch,px};}
const reg=(im,x0,y0,x1,y1)=>{let s=0,n=0;for(let y=y0;y<y1;y++)for(let x=x0;x<x1;x++){const i=(y*im.w+x)*im.ch;s+=0.299*im.px[i]+0.587*im.px[i+1]+0.114*im.px[i+2];n++;}return s/n;};
const g=png(process.argv[2]),p=png(process.argv[3]);
for(const [n,r] of Object.entries({room:[760,420,1060,560],wallL:[660,400,760,520],outsideR:[1400,300,1700,500],roadBR:[1200,800,1500,1000],dark:[100,600,500,900]}))console.log(n.padEnd(9),"game",reg(g,...r).toFixed(1),"port",reg(p,...r).toFixed(1));
