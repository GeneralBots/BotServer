/*****************************************************************************\
|                                               ( )_  _                       |
|    _ _    _ __   _ _    __    ___ ___     _ _ | ,_)(_)  ___   ___     _     |
|   ( '_`\ ( '__)/'_` ) /'_ `\/' _ ` _ `\ /'_` )| |  | |/',__)/' v `\ /'_`\   |
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
import { GBLog, GBMinInstance, GBService, IGBCoreService, GBDialogStep } from 'botlib';
import * as fs from 'fs';
import { GBDeployer } from './GBDeployer';
import { TSCompiler } from './TSCompiler';
import { CollectionUtil } from 'pragmatismo-io-framework';
const walkPromise = require('walk-promise');
import urlJoin = require('url-join');
import { DialogClass } from './GBAPIService';
import { Messages } from '../strings';
import { GBConversationalService } from './GBConversationalService';
//tslint:disable-next-line:no-submodule-imports
const vm = require('vm');
const vb2ts = require('vbscript-to-typescript/dist/converter');
const beautify = require('js-beautify').js;
var textract = require('textract');

/**
 * @fileoverview Virtualization services for emulation of BASIC.
 * This alpha version is using a hack in form of converter to
 * translate BASIC to TS and string replacements to emulate await code.
 * See http://jsfiddle.net/roderick/dym05hsy for more info on vb2ts, so
 * http://stevehanov.ca/blog/index.php?id=92 should be used to run it without
 * translation and enhance classic BASIC experience.
 */

/**
 * Basic services for BASIC manipulation.
 */
export class GBVMService extends GBService {
  public async loadDialogPackage(folder: string, min: GBMinInstance, core: IGBCoreService, deployer: GBDeployer) {
    const files = await walkPromise(folder);
    this.addHearDialog(min);

    await CollectionUtil.asyncForEach(files, async file => {
      if (!file) {
        return;
      }

      let filename: string = file.name;

      if (filename.endsWith('.docx')) {
        const wordFile = filename;
        const vbsFile = filename.substr(0, filename.indexOf('docx')) + 'vbs';
        const fullVbsFile = urlJoin(folder, vbsFile);
        const docxStat = fs.statSync(urlJoin(folder, wordFile));
        const interval = 30000; // If compiled is older 30 seconds, then recompile.
        let writeVBS = true;
        if (fs.existsSync(fullVbsFile)) {
          const vbsStat = fs.statSync(fullVbsFile);
          if (docxStat.mtimeMs < vbsStat.mtimeMs + interval) {
            writeVBS = false;
          }
        }
        if (writeVBS) {
          let text = await this.getTextFromWord(folder, wordFile);
          fs.writeFileSync(urlJoin(folder, vbsFile), text);
        }

        filename = vbsFile;

        let mainName = GBVMService.getMethodNameFromVBSFilename(filename);
        min.scriptMap[filename] = mainName;

        const fullFilename = urlJoin(folder, filename);
        // TODO: Implement in development mode, how swap for .vbs files
        // fs.watchFile(fullFilename, async () => {
        //   await this.run(fullFilename, min, deployer, mainName);
        // });

        const compiledAt = fs.statSync(fullFilename);
        const jsfile = urlJoin(folder, `${filename}.js`);

        if (fs.existsSync(jsfile)) {
          const jsStat = fs.statSync(jsfile);
          const interval = 30000; // If compiled is older 30 seconds, then recompile.
          if (compiledAt.isFile() && compiledAt.mtimeMs > jsStat.mtimeMs + interval) {
            await this.executeBASIC(fullFilename, min, deployer, mainName);
          } else {
            const parsedCode: string = fs.readFileSync(jsfile, 'utf8');
            this.executeJS(min, deployer, parsedCode, mainName);
          }
        } else {
          await this.executeBASIC(fullFilename, min, deployer, mainName);
        }
      }
    });
  }

  public static getMethodNameFromVBSFilename(filename: string) {
    let mainName = filename.replace(/\s|\-/gi, '').split('.')[0];
    return mainName.toLowerCase();
  }

  private async getTextFromWord(folder: string, filename: string) {
    return new Promise<string>(async (resolve, reject) => {
      textract.fromFileWithPath(urlJoin(folder, filename), { preserveLineBreaks: true }, (error, text) => {
        if (error) {
          reject(error);
        } else {
          text = text.replace('“', '"');
          text = text.replace('”', '"');
          text = text.replace('‘', "'");
          text = text.replace('’', "'");

          resolve(text);
        }
      });
    });
  }

  /**
   * Converts General Bots BASIC
   *
   *
   * @param code General Bots BASIC
   */
  public convertGBASICToVBS(code: string) {
    // Start and End of VB2TS tags of processing.

    code = `<%\n
    
    from = this.getFrom(step)
    today = this.getToday(step)
    now = this.getNow(step)
    id = sys().getRandomId()
    username = this.getUserName(step);
    mobile = this.getUserMobile(step);
    ubound = (list) => list.length;

    ${code}
    `;

    // Keywords from General Bots BASIC.

    code = code.replace(/(hear email)/gi, `email = askEmail()`);

    code = code.replace(/(hear)\s*(\w+)/gi, ($0, $1, $2) => {
      return `${$2} = hear()`;
    });

    code = code.replace(/(\w)\s*\=\s*find\s*(.*)/gi, ($0, $1, $2, $3) => {
      return `${$1} = sys().find(${$2})\n`;
    });

    code = code.replace(/(wait)\s*(\d+)/gi, ($0, $1, $2) => {
      return `sys().wait(${$2})`;
    });

    code = code.replace(/(get stock for )(.*)/gi, ($0, $1, $2) => {
      return `let stock = sys().getStock(${$2})`;
    });

    code = code.replace(/(\w+)\s*\=\s*get\s(.*)/gi, ($0, $1, $2) => {
      if ($2.indexOf('http') !== -1) {
        return `let ${$1} = sys().httpGet (${$2})`;
      } else {
        return `let ${$1} = sys().get (${$2})`;
      }
    });

    code = code.replace(/set\s(.*)/gi, ($0, $1, $2) => {
      return `sys().set (${$1})`;
    });

    code = code.replace(/(\w+)\s*\=\s*post\s*(.*),\s*(.*)/gi, ($0, $1, $2, $3) => {
      return `let ${$1} = sys().httpPost (${$2}, ${$3})`;
    });

    code = code.replace(/(create a bot farm using)(\s)(.*)/gi, ($0, $1, $2, $3) => {
      return `sys().createABotFarmUsing (${$3})`;
    });

    code = code.replace(/(transfer)/gi, () => {
      return `transfer (step)\n`;
    });

    code = code.replace(/(talk to)(\s)(.*)/gi, ($0, $1, $2, $3) => {
      return `sys().talkTo(${$3})\n`;
    });

    code = code.replace(/(talk)(\s)(.*)/gi, ($0, $1, $2, $3) => {
      return `talk (step, ${$3})\n`;
    });
    code = code.replace(/(send file)(\s*)(.*)/gi, ($0, $1, $2, $3) => {
      return `sendFile (step, ${$3})\n`;
    });
    code = code.replace(/(send file to)(\s*)(.*)/gi, ($0, $1, $2, $3) => {
      return `sendFileTo (step, ${$3})\n`;
    });

    code = code.replace(/(save)(\s)(.*)/gi, ($0, $1, $2, $3) => {
      return `sys().save(${$3})\n`;
    });

    code = `${code}\n%>`;

    return code;
  }

  public async executeBASIC(filename: any, min: GBMinInstance, deployer: GBDeployer, mainName: string) {
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
    tsCode = tsCode.replace(/export.*\n/gi, `export function ${mainName}(step:any) { let resolve;`);
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
        const promiseName = `promiseFor${variable}`;

        parsedCode = code.substring(pos, pos + match1.index);
        parsedCode += ``;
        parsedCode += `const ${promiseName}= async (step, ${variable}) => {`;
        parsedCode += `   return new Promise(async (resolve) => {`;

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
        parsedCode += '}\n';
        parsedCode += `hear (step, ${promiseName}, resolve);\n`;
        parsedCode += code.substring(pos + match1[0].length);

        // A interaction will be made for each hear.

        code = parsedCode;
      }

      parsedCode = this.handleThisAndAwait(parsedCode);

      parsedCode = beautify(parsedCode, { indent_size: 2, space_in_empty_paren: true });
      fs.writeFileSync(jsfile, parsedCode);

      this.executeJS(min, deployer, parsedCode, mainName);
      GBLog.info(`[GBVMService] Finished loading of ${filename}`);
    }
  }

  private executeJS(min: GBMinInstance, deployer: GBDeployer, parsedCode: string, mainName: string) {
    try {
      min.sandBoxMap[mainName.toLowerCase()] = parsedCode;
    } catch (error) {
      GBLog.error(`[GBVMService] ERROR loading ${error}`);
    }
  }

  private handleThisAndAwait(code: string) {
    // this insertion.

    code = code.replace(/sys\(\)/gi, 'this.sys()');
    code = code.replace(/("[^"]*"|'[^']*')|\btalk\b/gi, ($0, $1) => {
      return $1 === undefined ? 'this.talk' : $1;
    });
    code = code.replace(/("[^"]*"|'[^']*')|\bhear\b/gi, ($0, $1) => {
      return $1 === undefined ? 'this.hear' : $1;
    });
    code = code.replace(/("[^"]*"|'[^']*')|\bsendEmail\b/gi, ($0, $1) => {
      return $1 === undefined ? 'this.sendEmail' : $1;
    });
    code = code.replace(/("[^"]*"|'[^']*')|\baskEmail\b/gi, ($0, $1) => {
      return $1 === undefined ? 'this.askEmail' : $1;
    });
    code = code.replace(/("[^"]*"|'[^']*')|\bsendFile\b/gi, ($0, $1) => {
      return $1 === undefined ? 'this.sendFile' : $1;
    });
    code = code.replace(/("[^"]*"|'[^']*')|\btransfer\b/gi, ($0, $1) => {
      return $1 === undefined ? 'this.transfer' : $1;
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
          step.activeDialog.state.options = {};
          step.activeDialog.state.options.cbId = (step.options as any).id;
          step.activeDialog.state.options.previousResolve = (step.options as any).previousResolve;
          GBLog.info('BASIC: Asking for input (HEAR).');
          return await min.conversationalService.prompt(min, step, null);
        },
        async step => {
          const cbId = step.activeDialog.state.options.cbId;

          if (min.cbMap[cbId]) {
            const promise = min.cbMap[cbId].promise;
            delete min.cbMap[cbId];
            try {
              const opts = await promise(step, step.result);
              return await step.replaceDialog('/hear', opts);
            } catch (error) {
              GBLog.error(`Error running BASIC code: ${error}`);
              const locale = step.context.activity.locale;
              step.context.sendActivity(Messages[locale].very_sorry_about_error);
              return await step.replaceDialog('/ask', { isReturning: true });
            }
          } else {
            await step.replaceDialog('/ask', { isReturning: true });
          }
         }
      ])
    );
  }

  public static async callVM(text: string, min: GBMinInstance, step: GBDialogStep, deployer: GBDeployer) {
    const sandbox: DialogClass = new DialogClass(min, deployer);
    const context = vm.createContext(sandbox);
    const code = min.sandBoxMap[text];
    vm.runInContext(code, context);

    const mainMethod = text.toLowerCase();
    sandbox[mainMethod].bind(sandbox);
    return await sandbox[mainMethod](step);
  }
}
