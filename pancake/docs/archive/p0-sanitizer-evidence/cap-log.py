import sys
remaining=2*1024*1024
with open(sys.argv[1], 'wb') as output:
 while chunk:=sys.stdin.buffer.read(65536):
  if remaining:
   kept=chunk[:remaining];output.write(kept);remaining-=len(kept)
