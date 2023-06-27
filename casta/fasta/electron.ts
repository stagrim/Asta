import { app, BrowserWindow } from "electron";

async function createWindow() {
	let win = new BrowserWindow({
		autoHideMenuBar: true,
		kiosk: true
	});

	var onHeadersReceived = (d: any, c: any) => {
		if (d.responseHeaders['X-Frame-Options']) {
			delete d.responseHeaders['X-Frame-Options'];
		}
		c({ cancel: false, responseHeaders: d.responseHeaders });
	}
	win.webContents.session.webRequest.onHeadersReceived(onHeadersReceived);

	win.on('closed', function () {
		win.removeAllListeners();
	});

	await win.loadURL("http://127.0.0.1:3000/");
}

app.on("ready", createWindow);
