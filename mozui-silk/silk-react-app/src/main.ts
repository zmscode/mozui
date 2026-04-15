// Silk main process — app lifecycle and window management only.
// Business logic lives in the renderer (React).

Silk.onReady(() => {
	Silk.createWindow("main", {
		url: "./index.html",
		title: "Silk Todo",
		width: 460,
		height: 680,
		minWidth: 360,
		minHeight: 480,
		titlebarStyle: "hiddenInset",
		trafficLightPosition: { x: 8, y: 8 },
	});
});

// import { app, BrowserWindow } from "electron";
// import * as path from "node:path";

// const createWindow = (): void => {
// 	let mainWindow = new BrowserWindow({
// 		width: 800,
// 		height: 600,
// 		webPreferences: {
// 			preload: path.join(__dirname, "/preloads/preload.js"),
// 		},
// 	});

// 	mainWindow.loadFile("./windows/mainwindow.html");

// 	mainWindow.webContents.openDevTools();

// 	mainWindow.on("closed", () => {
// 		mainWindow = null;
// 	});
// };

// app.whenReady().then((): void => {
// 	createWindow();

// 	app.on("activate", (): void => {
// 		if (BrowserWindow.getAllWindows().length === 0) {
// 			createWindow();
// 		}
// 	});
// });

// app.on("window-all-closed", (): void => {
// 	if (process.platform !== "darwin") {
// 		app.quit();
// 	}
// });
