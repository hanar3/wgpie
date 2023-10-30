# wgpie
computer graphics is kinda hard

This is a research/exploration project that I'm using to learn and practice webgpu.
Technically this will compile to WASM and we can run this on a browser, but I haven't tested this yet.

Currently - the main branch implements the following:
[x] A model loader/renderer
  [] A material loader (?) (I still don't know how to reliably load the material from a blender model)
[x] A camera (move it with WASD) - it's a bit buggy (3d matrix math is hard)
[x] Support for instance-rendering! (quite fun to implement)

And inside the `2d` branch...well
I was mostly trying to build a ui-lib,the idea was to use tesselation to build 2d shapes for rects (and rounded rects)
It's currently using lyon as a tesselation lib, but not much progress has been made outside of rendering basic stuff, we can also do instancing on 2D.

There's also an ortographic camera which allows us scroll to zoom-in.
