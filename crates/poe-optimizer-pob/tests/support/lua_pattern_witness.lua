-- Test input generation only. All expected matches still come from original string.find.
-- Choose class members by asking original LuaJIT, never by querying the native matcher.
return function(pattern)
 local function closeClass(at)
  local j=at+1
  if pattern:sub(j,j)=='^'then j=j+1 end
  if pattern:sub(j,j)==']'then j=j+1 end
  while j<=#pattern do
   if pattern:sub(j,j)=='%'then j=j+2
   elseif pattern:sub(j,j)==']'then return j
   else j=j+1 end
  end
  return #pattern
 end
 local function member(class)
  for _,n in ipairs({55,97,65,32,95,37,45,10,0,255})do
   local s=string.char(n);local ok,a=pcall(string.find,s,class)
   if ok and a then return s end
  end
  for n=0,255 do
   local s=string.char(n);local ok,a=pcall(string.find,s,class)
   if ok and a then return s end
  end
  return ''
 end
 local out,stack,captures={}, {},{}
 local function emit(text)out[#out+1]=text end
 local i=1
 while i<=#pattern do
  local c=pattern:sub(i,i)
  if c=='^'and i==1 or c=='$'and i==#pattern then i=i+1
  elseif c=='%'then
   local next=pattern:sub(i+1,i+1)
   if next=='b'then emit(pattern:sub(i+2,i+2)..'a'..pattern:sub(i+3,i+3));i=i+4
   elseif next=='f'and pattern:sub(i+2,i+2)=='['then i=closeClass(i+2)+1
   elseif next:match('%d')then emit(captures[tonumber(next)]or'');i=i+2
   elseif next:match('%a')then emit(member('%'..next));i=i+2
   else emit(next);i=i+2 end
  elseif c=='['then local j=closeClass(i);emit(member(pattern:sub(i,j)));i=j+1
  elseif c=='('then
   local id=#captures+1;captures[id]=''
   if pattern:sub(i+1,i+1)==')'then i=i+2
   else stack[#stack+1]={id=id,start=#out+1};i=i+1 end
  elseif c==')'then
   local frame=table.remove(stack)
   if frame then captures[frame.id]=table.concat(out,'',frame.start)end
   i=i+1
  elseif c=='.'then emit('a');i=i+1
  elseif c=='*'or c=='+'or c=='-'or c=='?'then i=i+1
  else emit(c);i=i+1 end
 end
 return table.concat(out)
end
