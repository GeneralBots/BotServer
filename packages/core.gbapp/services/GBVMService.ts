/*****************************************************************************\
|                                               ( )_  _                       |
|    _ _    _ __   _ _    __    ___ ___     _ _ | ,_)(_)  ___   ___     _     |
|   ( '_`\ ( '__)/'_` ) /'_ `\/' _ ` _ `\ /'_` )| |  | |/',__)/' _ `\ /'_`\   |
|   | (_) )| |  ( (_| |( (_) || ( ) ( ) |( (_| || |_ | |\__, \| (˅) |( (_) )  |
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

import { WaterfallDialog } from 'botbuilder-dialogs';
import { GBLog, GBMinInstance, GBService, IGBCoreService } from 'botlib';
import * as fs from 'fs';
import { GBDeployer } from './GBDeployer';
import { TSCompiler } from './TSCompiler';

const walkPromise = require('walk-promise');

const vm = require('vm');
import urlJoin = require('url-join');
import { DialogClass } from './GBAPIService';
//tslint:disable-next-line:no-submodule-imports
const vb2ts = require('vbscript-to-typescript/dist/converter');
const beautify = require('js-beautify').js;

/**
 * @fileoverview Virtualization services for emulation of BASIC.
 * This alpha version is using a hack in form of converter to
 * translate BASIC to TSand string replacements to emulate await code.
 * See http://jsfiddle.net/roderick/dym05hsy for more info on vb2ts, so
 * http://stevehanov.ca/blog/index.php?id=92 should be used to run it without
 * translation and enhance classic BASIC experience.
 */

/**
 * Basic services for BASIC manipulation.
 */
export class GBVMService extends GBService {
  private readonly script = new vm.Script();

  public async loadDialogPackage(folder: string, min: GBMinInstance, core: IGBCoreService, deployer: GBDeployer) {
    const files = await walkPromise(folder);
    this.addHearDialog(min);

    return Promise.all(
      files.map(async file => {
        if (
          file.name.endsWith('.vbs') ||
          file.name.endsWith('.vb') ||
          file.name.endsWith('.basic') ||
          file.name.endsWith('.bas')
        ) {
          const mainName = file.name.replace(/\-|\./g, '');
          min.scriptMap[file.name] = mainName;

          const filename = urlJoin(folder, file.name);
          fs.watchFile(filename, async () => {
            await this.run(filename, min, deployer, mainName);
          });

          await this.run(filename, min, deployer, mainName);
        }
      })
    );
  }

  /**
   * Converts General Bots BASIC
   *
   *
   * @param code General Bots BASIC
   */
  public convertGBASICToVBS(code: string) {
    // Start and End of VB2TS tags of processing.

    code = `<%\n${code}`;

    // Keywords from General Bots BASIC.

    code = code.replace(/(hear)\s*(\w+)/g, ($0, $1, $2) => {
      return `${$2} = hear()`;
    });

    code = code.replace(/(wait)\s*(\d+)/g, ($0, $1, $2) => {
      return `sys().wait(${$2})`;
    });

    code = code.replace(/(generate a password)/g, ($0, $1) => {
      return 'let password = sys().generatePassword()';
    });

    code = code.replace(/(get)(\s)(.*)/g, ($0, $1, $2) => {
      return `sys().httpGet (${$2})`;
    });

    code = code.replace(/(create a bot farm using)(\s)(.*)/g, ($0, $1, $2, $3) => {
      return `sys().createABotFarmUsing (${$3})`;
    });

    code = code.replace(/(talk)(\s)(.*)/g, ($0, $1, $2, $3) => {
      return `talk (${$3})\n`;
    });

    code = `${code}\n%>`;

    return code;
  }

  public async run(filename: any, min: GBMinInstance, deployer: GBDeployer, mainName: string) {
    // Converts General Bots BASIC into regular VBS

    const basicCode: string = fs.readFileSync(filename, 'utf8');
    const vbsCode = this.convertGBASICToVBS(basicCode);
    const vbsFile = `${filename}.compiled`;
    fs.writeFileSync(vbsFile, vbsCode, 'utf8');

    // Converts VBS into TS.
    vb2ts.convertFile(vbsFile);

    // Convert TS into JS.
    const tsfile: string = `${filename}.ts`;
    let tsCode: string = fs.readFileSync(tsfile, 'utf8');
    tsCode = tsCode.replace(/export.*\n/g, `export function ${mainName}() {`);
    fs.writeFileSync(tsfile, tsCode);

    const tsc = new TSCompiler();
    tsc.compile([tsfile]);

    // Run JS into the GB context.
    const jsfile = `${tsfile}.js`.replace('.ts', '');

    if (fs.existsSync(jsfile)) {
      let code: string = fs.readFileSync(jsfile, 'utf8');

      code = code.replace(/^.*exports.*$/gm, '');

      // Finds all hear calls.

      let parsedCode = code;
      const hearExp = /(\w+).*hear.*\(\)/;

      let match1;

      while ((match1 = hearExp.exec(code))) {
        let pos = 0;

        // Writes async body.

        const variable = match1[1]; // Construct variable = hear ().

        parsedCode = code.substring(pos, pos + match1.index);
        parsedCode += `hear (async (${variable}) => {\n`;

        // Skips old construction and point to the async block.

        pos = pos + match1.index;
        let tempCode = code.substring(pos + match1[0].length + 1);
        const start = pos;

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

      parsedCode = this.handleThisAndAwait(parsedCode);

      parsedCode = beautify(parsedCode, { indent_size: 2, space_in_empty_paren: true })
      fs.writeFileSync(jsfile, parsedCode);

      const sandbox: DialogClass = new DialogClass(min, deployer);
      const context = vm.createContext(sandbox);
      vm.runInContext(parsedCode, context);
      min.sandBoxMap[mainName] = sandbox;
      GBLog.info(`[GBVMService] Finished loading of ${filename}`);

    }
  }

  private handleThisAndAwait(code: string) {
    // this insertion.

    code = code.replace(/sys\(\)/g, 'this.sys()');
    code = code.replace(/("[^"]*"|'[^']*')|\btalk\b/g, ($0, $1) => {
      return $1 === undefined ? 'this.talk' : $1;
    });
    code = code.replace(/("[^"]*"|'[^']*')|\bhear\b/g, ($0, $1) => {
      return $1 === undefined ? 'this.hear' : $1;
    });
    code = code.replace(/("[^"]*"|'[^']*')|\bsendEmail\b/g, ($0, $1) => {
      return $1 === undefined ? 'this.sendEmail' : $1;
    });

    // await insertion.

    code = code.replace(/this\./gm, 'await this.');
    code = code.replace(/function/gm, 'async function');

    return code;
  }

  private addHearDialog(min) {
    min.dialogs.add(
      new WaterfallDialog('/hear', [
        async step => {
          step.activeDialog.state.cbId = (step.options as any).id;

          return await step.prompt('textPrompt', {});
        },
        async step => {
          const cbId = step.activeDialog.state.cbId;
          const cb = min.cbMap[cbId];
          cb.bind({ step: step, context: step.context });
          await cb(step.result);
          return await step.next();
        }
      ])
    );
  }
}
