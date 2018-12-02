/*****************************************************************************\
|                                               ( )_  _                       |
|    _ _    _ __   _ _    __    ___ ___     _ _ | ,_)(_)  ___   ___     _     |
|   ( '_`\ ( '__)/'_` ) /'_ `\/' _ ` _ `\ /'_` )| |  | |/',__)/' _ `\ /'_`\   |
|   | (_) )| |  ( (_| |( (_) || ( ) ( ) |( (_| || |_ | |\__, \| ( ) |( (_) )  |
|   | ,__/'(_)  `\__,_)`\__  |(_) (_) (_)`\__,_)`\__)(_)(____/(_) (_)`\___/'  |
|   | |                ( )_) |                                                |
|   (_)                 \___/'                                                |
|                                                                             |
| General Bots Copyright (c) Pragmatismo.io. All rights reserved.             |
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
| "General Bots" is a registered trademark of Pragmatismo.io.                 |
| The licensing of the program under the AGPLv3 does not imply a              |
| trademark license. Therefore any rights, title and interest in              |
| our trademarks remain entirely with us.                                     |
|                                                                             |
\*****************************************************************************/

'use strict';

import { GBMinInstance, IGBCoreService } from 'botlib';
import * as fs from 'fs';
import { DialogClass } from './GBAPIService';
import { GBDeployer } from './GBDeployer';
import { TSCompiler } from './TSCompiler';
import { WaterfallDialog } from 'botbuilder-dialogs';
const util = require('util');
const logger = require('../../../src/logger');
const vm = require('vm');
const UrlJoin = require('url-join');
const vb2ts = require('vbscript-to-typescript/dist/converter');

/**
 * @fileoverview Virtualization services for emulation of BASIC.
 */

export class GBVMService implements IGBCoreService {
  private script = new vm.Script();

  public async loadJS(
    filename: string,
    min: GBMinInstance,
    core: IGBCoreService,
    deployer: GBDeployer,
    localPath: string
  ): Promise<void> {
    const path = 'packages/default.gbdialog';
    const file = 'bot.vbs';
    const source = UrlJoin(path, file);

    // Example when handled through fs.watch() listener
    fs.watchFile(source, async (curr, prev) => {
      await this.run(source, path, min, deployer, filename);
    });
    await this.run(source, path, min, deployer, filename);
    this.addHearDialog(min);
  }

  public async run(source: any, path: string, min: any, deployer: GBDeployer, filename: string) {
    // Converts VBS into TS.

    vb2ts.convertFile(source);

    // Convert TS into JS.
    const tsfile = `bot.ts`;
    const tsc = new TSCompiler();
    tsc.compile([UrlJoin(path, tsfile)]);

    // Run JS into the GB context.
    const jsfile = `bot.js`;
    let localPath = UrlJoin(path, jsfile);

    if (fs.existsSync(localPath)) {
      let code: string = fs.readFileSync(localPath, 'utf8');
      code = code.replace(/^.*exports.*$/gm, '');

      // Finds all hear calls.

      let parsedCode = code;
      let hearExp = /(\w+).*hear.*\(\)/;

      let match1;

      while ((match1 = hearExp.exec(code))) {

        let pos = 0;

        // Writes async body.

        const variable = match1[1]; // variable = hear();

        parsedCode = code.substring(pos, pos + match1.index);
        parsedCode += `hear (async (${variable}) => {\n`;

        // Skips old construction and point to the async block.

        pos = pos + match1.index;
        let tempCode = code.substring(pos + match1[0].length + 1);
        let start = pos;

        // Balances code blocks and checks for exits.

        let right = 0;
        let left = 1;
        let match2;
        while ((match2 = /\{|\}/.exec(tempCode))) {
          const c = tempCode.substring(match2.index, match2.index + 1);

          if (c === '}') {
            right++;
          } else if (c === '{') {
            left++;
          }

          tempCode = tempCode.substring(match2.index + 1);
          pos += match2.index + 1;

          if (left === right) {
            break;
          }
        }

        parsedCode += code.substring(start + match1[0].length + 1, pos + match1[0].length);
        parsedCode += '});\n';
        parsedCode += code.substring(pos + match1[0].length);

        // A interaction will be made for each hear.

        code = parsedCode;
      }

      parsedCode = parsedCode.replace(/("[^"]*"|'[^']*')|\btalk\b/g, function($0, $1) {
        return $1 == undefined ? 'this.talk' : $1;
      });

      parsedCode = parsedCode.replace(/("[^"]*"|'[^']*')|\bhear\b/g, function($0, $1) {
        return $1 == undefined ? 'this.hear' : $1;
      });

      parsedCode = parsedCode.replace(/("[^"]*"|'[^']*')|\bsendEmail\b/g, function($0, $1) {
        return $1 == undefined ? 'this.sendEmail' : $1;
      });

      parsedCode = parsedCode.replace(/this\./gm, 'await this.');
      parsedCode = parsedCode.replace(/function/gm, 'async function');

      fs.writeFileSync(localPath, parsedCode);

      const sandbox: DialogClass = new DialogClass(min);
      const context = vm.createContext(sandbox);
      vm.runInContext(parsedCode, context);
      min.sandbox = sandbox;
      await deployer.deployScriptToStorage(min.instanceId, filename);
      logger.info(`[GBVMService] Finished loading of ${filename}`);
    }
  }

  private addHearDialog(min) {
    min.dialogs.add(
      new WaterfallDialog('/hear', [
        async step => {
          step.activeDialog.state.cbId = step.options['id'];
          step.activeDialog.state.idResolve = step.options['idResolve'];

          return await step.prompt('textPrompt', {});
        },
        async step => {
          min.sandbox.context = step.context;
          min.sandbox.step = step;

          const cbId = step.activeDialog.state.cbId;
          const cb = min.cbMap[cbId];
          cb.bind({ step: step, context: step.context }); // TODO: Necessary or min.sandbox
          await cb();

          return await step.next();
        }
      ])
    );
  }
}
