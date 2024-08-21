import { GBConfigService } from "../packages/core.gbapp/services/GBConfigService";

const { app, BrowserWindow } = require('electron');
const path = require('path');
const url = require('url');

function createWindow() {
  // Create the browser window.
  const win = new BrowserWindow({ width: 800, height: 600 });

  // and load the index.html of the app.
  win.loadURL(
    url.format({
      pathname: path.join(__dirname, `http://localhost:${GBConfigService.get('PORT')}`),
      protocol: 'file:',
      slashes: true
    })
  );
}

app.on('ready', createWindow);
