import AppKit
let image=NSImage(size:NSSize(width:1024,height:1024))
image.lockFocus()
let rect=NSRect(x:0,y:0,width:1024,height:1024)
let base=NSBezierPath(roundedRect:rect.insetBy(dx:24,dy:24),xRadius:210,yRadius:210)
NSGradient(starting:NSColor(calibratedRed:0.10,green:0.16,blue:0.22,alpha:1),ending:NSColor(calibratedRed:0.025,green:0.055,blue:0.085,alpha:1))!.draw(in:base,angle:270)
NSColor(calibratedWhite:1,alpha:0.09).setStroke();base.lineWidth=8;base.stroke()
for (i,level) in [360.0,660.0,475.0].enumerated(){
 let x=CGFloat(260+i*252)
 let track=NSBezierPath(roundedRect:NSRect(x:x-23,y:208,width:46,height:608),xRadius:23,yRadius:23)
 NSColor(calibratedRed:0.20,green:0.92,blue:0.73,alpha:0.24).setFill();track.fill()
 let active=NSBezierPath(roundedRect:NSRect(x:x-23,y:208,width:46,height:level-208),xRadius:23,yRadius:23)
 NSColor(calibratedRed:0.29,green:0.95,blue:0.75,alpha:1).setFill();active.fill()
 let handle=NSBezierPath(roundedRect:NSRect(x:x-86,y:level-43,width:172,height:86),xRadius:30,yRadius:30)
 (i==1 ? NSColor(calibratedRed:1,green:0.73,blue:0.34,alpha:1):NSColor(calibratedRed:0.85,green:0.99,blue:0.96,alpha:1)).setFill();handle.fill()
}
image.unlockFocus()
let png=NSBitmapImageRep(data:image.tiffRepresentation!)!.representation(using:.png,properties:[:])!
try png.write(to:URL(fileURLWithPath:CommandLine.arguments[1]))
