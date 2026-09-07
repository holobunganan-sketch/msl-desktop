export function reportBlocks(content:string|null):Array<{headline:string;body:string}> {
  const blocks:Array<{headline:string;body:string}>=[];
  for(const raw of (content??'').split(/\r?\n/)){
    const line=raw.trim();if(!line)continue;
    if(/^\d+[.)、]\s+/.test(line)||!blocks.length){
      blocks.push({headline:line.replace(/^\d+[.)、]\s+/,''),body:''});
    }else{
      const last=blocks[blocks.length-1];last.body+=(last.body?'\n':'')+line;
    }
  }
  return blocks;
}
