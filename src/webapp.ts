import { GBConfigService } from '../packages/core.gbapp/services/GBConfigService.js';
const {app} = (await import('electron')).default;

import path from 'path';
import url from 'url';
 
export function runUI() {
  // Create the browser window.
  const win = null;// new BrowserWindow({ width: 800, height: 600, title: 'General Bots Studio' });

  import('./app.js').then(gb => {
    gb.GBServer.run();
    // and load the index.html of the app.
    win.loadURL(
      url.format({
        pathname: path.join(__dirname, `http://localhost:${GBConfigService.get('PORT')}`),
        protocol: 'file:',
        slashes: true
      })
    );
  });
}

export class GBUI {
  static run() {
    app.on('ready', runUI);
  }
}
