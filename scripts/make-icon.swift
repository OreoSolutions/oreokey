// Sinh icon app từ ảnh nguồn: cắt viền, bo góc squircle, nền trong suốt.
// Chạy: swift scripts/make-icon.swift <in.png> <out.png> [crop%] [radius%]
import AppKit

let args = CommandLine.arguments
guard args.count >= 3, let src = NSImage(contentsOfFile: args[1]) else {
    fputs("usage: make-icon.swift in.png out.png [crop%] [radius%]\n", stderr)
    exit(1)
}
let cropPct = args.count > 3 ? Double(args[3]) ?? 0.08 : 0.08
let radiusPct = args.count > 4 ? Double(args[4]) ?? 0.225 : 0.225

let size = 1024.0
let canvas = NSImage(size: NSSize(width: size, height: size))
canvas.lockFocus()

let rect = NSRect(x: 0, y: 0, width: size, height: size)
NSBezierPath(roundedRect: rect, xRadius: size * radiusPct, yRadius: size * radiusPct)
    .addClip()

// Cắt bỏ viền trắng quanh thẻ logo rồi phóng đầy canvas.
let w = src.size.width
let inset = w * cropPct
let srcRect = NSRect(x: inset, y: inset, width: w - 2 * inset, height: w - 2 * inset)
src.draw(in: rect, from: srcRect, operation: .copy, fraction: 1.0)
canvas.unlockFocus()

guard let tiff = canvas.tiffRepresentation,
      let rep = NSBitmapImageRep(data: tiff),
      let png = rep.representation(using: .png, properties: [:])
else { exit(2) }
try! png.write(to: URL(fileURLWithPath: args[2]))
