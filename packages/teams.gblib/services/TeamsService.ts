/*****************************************************************************\
|  █████  █████ ██    █ █████ █████   ████  ██      ████   █████ █████  ███ ® |
| ██      █     ███   █ █     ██  ██ ██  ██ ██      ██  █ ██   ██  █   █      |
| ██  ███ ████  █ ██  █ ████  █████  ██████ ██      ████   █   █   █    ██    |
| ██   ██ █     █  ██ █ █     ██  ██ ██  ██ ██      ██  █ ██   ██  █      █   |
|  █████  █████ █   ███ █████ ██  ██ ██  ██ █████   ████   █████   █   ███    |
|                                                                             |
| General Bots Copyright (c) pragmatismo.com.br. All rights reserved.             |
| Licensed under the AGPL-3.0.                                                |
|                                                                             |
| According to our dual licensing model, this program can be used either      |
| under the terms of the GNU Affero General Public License, version 3,        |
| or under a proprietary license.                                             |
|                                                                             |
| The texts of the GNU Affero General Public License with an additional       |
| permission and of our proprietary license can be found at and               |
| in the LICENSE file you have received along with this program.              |
|                                                                             |
| This program is distributed in the hope that it will be useful,             |
| but WITHOUT ANY WARRANTY, without even the implied warranty of              |
| MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the                |
| GNU Affero General Public License for more details.                         |
|                                                                             |
| "General Bots" is a registered trademark of pragmatismo.com.br.             |
| The licensing of the program under the AGPLv3 does not imply a              |
| trademark license. Therefore any rights, title and interest in              |
| our trademarks remain entirely with us.                                     |
|                                                                             |
\*****************************************************************************/

import { GBService } from 'botlib';
import Fs from 'fs';
import AdmZip from 'adm-zip';

/**
 * Support for Whatsapp.
 */
export class TeamsService extends GBService {
  public async getAppFile(manifest) {
    var zip = new AdmZip();
    zip.addFile('manifest.json', Buffer.from(manifest, 'utf8'), 'Built with General Bots™.');
    zip.addLocalFile('teams-color.png', null, 'color.png');
    zip.addLocalFile('teams-outline.png', null, 'outline.png');
    return zip.toBuffer();
  }

  public async getManifest(marketplaceId, botName, botDescription, id, packageName, yourName) {
    let content = Fs.readFileSync('teams-manifest.json', 'utf8');

    content = content.replace(/\@\@marketplaceId/gi, marketplaceId);
    content = content.replace(/\@\@botName/gi, botName);
    content = content.replace(/\@\@botDescription/gi, botDescription);
    content = content.replace(/\@\@id/gi, id);
    content = content.replace(/\@\@packageName/gi, packageName);
    content = content.replace(/\@\@yourName/gi, yourName);

    return content;
  }
}
