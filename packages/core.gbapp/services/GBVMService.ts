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

    //vb2ts.convertFile(source);

    // Convert TS into JS.
    const tsfile = `bot.ts`;
    const tsc = new TSCompiler();
    //tsc.compile([UrlJoin(path, tsfile)]);
    // Run JS into the GB context.
    const jsfile = `bot.js`;
    let localPath = UrlJoin(path, jsfile);

    if (fs.existsSync(localPath)) {
      let code: string = fs.readFileSync(localPath, 'utf8');
      code = code.replace(/^.*exports.*$/gm, '');
      let match1;
      let match2;
      let finalCode = '';
      let pos = 0;
      let nextCode = code;
      let hearExp = /(\w+).*hear.*\(\)/;

      while (match1 = hearExp.exec(nextCode)) {

        // Write async body.

        const variable = match1[1]; // variable = hear();

        finalCode += code.substring(pos, pos + match1.index);
        finalCode += `hear (async (${variable}) => {\n`;

        // Skip old construction and point to the async block.

        pos = pos + match1.index;
        nextCode = code.substring(pos + match1[0].length + 1);
        let start = pos;

        // Find last }

        let right = 0;
        let left = 1;
        while ((match2 = /\{|\}/.exec(nextCode))) {
          const c = nextCode.substring(match2.index, match2.index + 1);

          if (c === '}') {
            right++;
          } else if (c === '{') {
            left++;
          }

          let match3
          if (match3 = hearExp.exec(nextCode))
          {
            nextCode = nextCode.substring(match3.index + 1);
            pos += match3.index;
            break;
          }

          nextCode = nextCode.substring(match2.index + 1);
          pos += match2.index + 1;

          if (left === right) {
            break;
          }

        }

        finalCode += code.substring(start + match1[0].length + 1, pos + match1[0].length);
        finalCode += '});\n';

        nextCode = code.substring(pos +  match1[0].length);
      }

      finalCode = finalCode.replace(/("[^"]*"|'[^']*')|\btalk\b/g, function($0, $1) {
        return $1 == undefined ? 'this.talk' : $1;
      });

      finalCode = finalCode.replace(/("[^"]*"|'[^']*')|\bhear\b/g, function($0, $1) {
        return $1 == undefined ? 'this.hear' : $1;
      });

      finalCode = finalCode.replace(/("[^"]*"|'[^']*')|\bsendEmail\b/g, function($0, $1) {
        return $1 == undefined ? 'this.sendEmail' : $1;
      });

      finalCode = finalCode.replace(/this\./gm, 'await this.');
      finalCode = finalCode.replace(/function/gm, 'async function');
      console.log(finalCode);

      const sandbox: DialogClass = new DialogClass(min);
      const context = vm.createContext(sandbox);
      vm.runInContext(finalCode, context);
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
