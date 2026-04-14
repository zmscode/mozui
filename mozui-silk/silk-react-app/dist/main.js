// src/main.ts
Silk.onReady(() => {
  Silk.createWindow("main", {
    url: "./index.html",
    title: "Silk Todo",
    width: 460,
    height: 680,
    minWidth: 360,
    minHeight: 480,
    titlebarStyle: "hiddenInset",
    trafficLightPosition: { x: 0, y: 0 }
  });
});
