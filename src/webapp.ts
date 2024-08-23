import { GBConfigService } from "../packages/core.gbapp/services/GBConfigService";
import { app, BrowserWindow } from 'electron';
import path from 'path';
import url from 'url';

 export function runUI() {

  // Create the browser window.
  const win = new BrowserWindow({ width: 800, height: 600, title: 'General Bots Studio' });

  // and load the index.html of the app.
  win.loadURL(
    url.format({
      pathname: path.join(__dirname, `http://localhost:${GBConfigService.get('PORT')}`),
      protocol: 'file:',
      slashes: true
    })
  );
}

app.on('ready', runUI);
