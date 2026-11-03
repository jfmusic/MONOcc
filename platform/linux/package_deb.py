#!/usr/bin/env python3
"""Build a Debian test package without requiring dpkg on the build host."""
from pathlib import Path
import gzip,io,sys,tarfile,time
root=Path(__file__).resolve().parent
publish,out=map(Path,sys.argv[1:]);out.parent.mkdir(parents=True,exist_ok=True)
app=publish/'MONOcc'
if not app.is_file() or app.read_bytes()[:4]!=b'\x7fELF':raise SystemExit('Publish a Linux ELF executable first.')
files={
 './opt/monocc/MONOcc':(app.read_bytes(),0o755),
 './usr/bin/monocc':('/opt/monocc/MONOcc',0o777),
 './usr/share/applications/MONOcc.desktop':(b'[Desktop Entry]\nType=Application\nName=MONOcc Preview\nComment=Set up Monogram controls\nExec=/opt/monocc/MONOcc\nTryExec=/opt/monocc/MONOcc\nIcon=monocc\nTerminal=false\nCategories=Utility;\n',0o644),
 './usr/share/icons/hicolor/128x128/apps/monocc.png':((root/'MONOcc.png').read_bytes(),0o644),
 './usr/lib/udev/rules.d/70-monocc.rules':((root/'70-monocc.rules').read_bytes(),0o644),
 './usr/share/doc/monocc-preview/copyright':((root/'../../LICENSE').read_bytes(),0o644),
 './usr/share/doc/monocc-preview/README':((root/'README.md').read_bytes(),0o644),
}
notice_dir=root/'notices'
if notice_dir.exists():
 for p in notice_dir.iterdir():
  if p.is_file():files['./usr/share/doc/monocc-preview/'+p.name]=(p.read_bytes(),0o644)
def tar(entries):
 b=io.BytesIO()
 with tarfile.open(fileobj=b,mode='w',format=tarfile.GNU_FORMAT) as t:
  for name,(data,mode) in entries.items():
   info=tarfile.TarInfo(name);info.mode=mode;info.uid=info.gid=0;info.uname=info.gname='root';info.mtime=int(time.time())
   if isinstance(data,str):info.type=tarfile.SYMTYPE;info.linkname=data;t.addfile(info)
   else:info.size=len(data);t.addfile(info,io.BytesIO(data))
 return gzip.compress(b.getvalue(),compresslevel=6,mtime=0)
control=f'''Package: monocc-preview
Version: 0.2.0~preview1
Section: utils
Priority: optional
Architecture: amd64
Maintainer: MONOcc contributors
Installed-Size: {(app.stat().st_size+1023)//1024}
Depends: libc6 (>= 2.35), libstdc++6, libicu74, libssl3t64, libx11-6, libxtst6, libfontconfig1, libfreetype6, libice6, libsm6, libxrandr2, libxi6, libxcursor1
Description: MONOcc Linux contributor preview for CoreV2
 Initial Linux port targeting Ubuntu 24.04 amd64 in an X11 desktop session.
 Background controller and setup UI; native Wayland automation not implemented.
 Hardware and installation tests are still required before public release.
'''.encode()
post=b'#!/bin/sh\nset -e\nif command -v udevadm >/dev/null 2>&1; then udevadm control --reload-rules || true; fi\nexit 0\n'
parts=[('debian-binary',b'2.0\n'),('control.tar.gz',tar({'./control':(control,0o644),'./postinst':(post,0o755),'./postrm':(post,0o755)})),('data.tar.gz',tar(files))]
with out.open('wb') as f:
 f.write(b'!<arch>\n')
 for name,data in parts:
  header=f'{name+"/":<16}{int(time.time()):<12}{0:<6}{0:<6}{"100644":<8}{len(data):<10}`\n'.encode('ascii')
  assert len(header)==60
  f.write(header);f.write(data)
  if len(data)%2:f.write(b'\n')
print(out)
