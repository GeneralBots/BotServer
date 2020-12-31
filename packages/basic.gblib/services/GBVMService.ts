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
import { GBDeployer } from '../../core.gbapp/services/GBDeployer';
import { TSCompiler } from './TSCompiler';
import { CollectionUtil } from 'pragmatismo-io-framework';
import urlJoin = require('url-join');
import { DialogKeywords } from './DialogKeywords';
import { Messages } from '../strings';
import { GBConversationalService } from '../../core.gbapp/services/GBConversationalService';
//tslint:disable-next-line:no-submodule-imports
const vm = require('vm');
const vb2ts = require('./vbscript-to-typescript');
const beautify = require('js-beautify').js;
const textract = require('textract');
const walkPromise = require('walk-promise');
const phoneUtil = require('google-libphonenumber').PhoneNumberUtil.getInstance();
const phone = require('phone');

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
    
    id = sys().getRandomId()
    username = this.getUserName(step);
    mobile = this.getUserMobile(step);
    from = mobile;
    ubound = function(list){return list.length};
    

    ${code}
    `;

    // Keywords from General Bots BASIC.

    code = code.replace(/hear (\w+) as email/gi, ($0, $1) => {
      return `${$1} = hear("email")`;
    });

    code = code.replace(/hear (\w+) as integer/gi, ($0, $1, $2) => {
      return `${$1} = hear("integer")`;
    });

    code = code.replace(/hear (\w+) as boolean/gi, ($0, $1, $2) => {
      return `${$1} = hear("boolean")`;
    });

    code = code.replace(/hear (\w+) as name/gi, ($0, $1, $2) => {
      return `${$1} = hear("name")`;
    });

    code = code.replace(/hear (\w+) as date/gi, ($0, $1, $2) => {
      return `${$1} = hear("date")`;
    });

    code = code.replace(/hear (\w+) as hour/gi, ($0, $1, $2) => {
      return `${$1} = hear("hour")`;
    });

    code = code.replace(/hear (\w+) as phone/gi, ($0, $1, $2) => {
      return `${$1} = hear("phone")`;
    });

    code = code.replace(/hear (\w+) as money/gi, ($0, $1, $2) => {
      return `${$1} = hear("money")`;
    });

    code = code.replace(/hear (\w+) as language/gi, ($0, $1, $2) => {
      return `${$1} = hear("language")`;
    });

    code = code.replace(/hear (\w+) as zipcode/gi, ($0, $1, $2) => {
      return `${$1} = hear("zipcode")`;
    });

    code = code.replace(/hear (\w+) as (.*)/gi, ($0, $1, $2) => {
      return `${$1} = hear("menu", ${$2})`;
    });

    code = code.replace(/(hear on)(\s)(.*)/gi, ($0, $1, $2, $3) => {
      return `sys().gotoDialog(${$3})\n`;
    });

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

    code = code.replace(/(set language)(\s*)(.*)/gi, ($0, $1, $2, $3) => {
      return `setLanguage (step, ${$3})\n`;
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

    code = code.replace(/(exit)/gi, () => {
      return `resolve();\n`;
    });

    code = code.replace(/(show menu)/gi, () => {
      return `showMenu (step)\n`;
    });

    code = code.replace(/(talk to)(\s)(.*)/gi, ($0, $1, $2, $3) => {
      return `sys().talkTo(${$3})\n`;
    });

    code = code.replace(/(talk)(\s)(.*)/gi, ($0, $1, $2, $3) => {
      return `talk (step, ${$3})\n`;
    });

    code = code.replace(/(send sms to)(\s*)(.*)/gi, ($0, $1, $2, $3) => {
      return `sys().sendSmsTo (${$3})\n`;
    });

    code = code.replace(/(send file to)(\s*)(.*)/gi, ($0, $1, $2, $3) => {
      return `sendFileTo (step, ${$3})\n`;
    });

    code = code.replace(/(send file)(\s*)(.*)/gi, ($0, $1, $2, $3) => {
      return `sendFile (step, ${$3})\n`;
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
      const hearExp = /(\w+).*hear.*\((.*)\)/;

      let match1;

      while ((match1 = hearExp.exec(code))) {
        let pos = 0;

        // Writes async body.

        const variable = match1[1]; // Construct variable = hear ().
        const args = match1[2]; // Construct variable = hear ("A", "B").
        const promiseName = `promiseFor${variable}`;

        parsedCode = code.substring(pos, pos + match1.index);
        parsedCode += ``;
        parsedCode += `const ${promiseName}= async (step, ${variable}) => {`;
        parsedCode += `   return new Promise(async (resolve, reject) => { try {`;

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

        parsedCode += '}catch(error){reject(error);}});\n';
        parsedCode += '}\n';


        parsedCode += `hear (step, ${promiseName}, resolve, ${args === '' ? null : args});\n`;
        parsedCode += code.substring(pos + match1[0].length);

        // A interaction will be made for each hear.

        code = parsedCode;
      }

      parsedCode = this.handleThisAndAwait(parsedCode);

      parsedCode = parsedCode.replace(/(now)(?=(?:[^"]|"[^"]*")*$)/gi, 'await this.getNow(step)');
      parsedCode = parsedCode.replace(/(today)(?=(?:[^"]|"[^"]*")*$)/gi, 'await this.getToday(step)');

      parsedCode = beautify(parsedCode, { indent_size: 2, space_in_empty_paren: true });
      fs.writeFileSync(jsfile, parsedCode);

      this.executeJS(min, deployer, parsedCode, mainName);
      GBLog.info(`[GBVMService] Finished loading of ${filename}, JavaScript from Word: \n ${parsedCode}`);
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
    code = code.replace(/("[^"]*"|'[^']*')|\bsendFileTo\b/gi, ($0, $1) => {
      return $1 === undefined ? 'this.sendFileTo' : $1;
    });
    code = code.replace(/("[^"]*"|'[^']*')|\bsendFile\b/gi, ($0, $1) => {
      return $1 === undefined ? 'this.sendFile' : $1;
    });
    code = code.replace(/("[^"]*"|'[^']*')|\bsetLanguage\b/gi, ($0, $1) => {
      return $1 === undefined ? 'this.setLanguage' : $1;
    });
    code = code.replace(/("[^"]*"|'[^']*')|\btransfer\b/gi, ($0, $1) => {
      return $1 === undefined ? 'this.transfer' : $1;
    });
    code = code.replace(/("[^"]*"|'[^']*')|\bmenu\b/gi, ($0, $1) => {
      return $1 === undefined ? 'this.menu' : $1;
    });

    // await insertion.

    code = code.replace(/this\./gm, 'await this.');
    code = code.replace(/function/gm, 'async function');
    code = code.replace('ubound = async', 'ubound =');  // TODO: Improve this.

    return code;
  }

  private addHearDialog(min) {
    min.dialogs.add(
      new WaterfallDialog('/hear', [
        async step => {
          step.activeDialog.state.options = step.options;
          step.activeDialog.state.options.id = (step.options as any).id;
          step.activeDialog.state.options.previousResolve = (step.options as any).previousResolve;

          if (step.options['args']) {

            GBLog.info(`BASIC: Asking for input (HEAR with ${step.options['args'][0]}).`);
          }
          else {

            GBLog.info('BASIC: Asking for input (HEAR).');
          }

          return await min.conversationalService.prompt(min, step, null);
        },
        async step => {

          const isIntentYes = (locale, utterance) => {
            return utterance.toLowerCase().match(Messages[locale].affirmative_sentences);
          }

          let result = step.result;
          if (step.activeDialog.state.options['kind'] === "boolean") {
            if (isIntentYes('pt-BR', step.result)) {
              result = true;
            }
            else {
              result = false;
            }
          }
          else if (step.activeDialog.state.options['kind'] === "email") {

            const extractEntity = (text) => {
              return text.match(/([a-zA-Z0-9._-]+@[a-zA-Z0-9._-]+\.[a-zA-Z0-9_-]+)/gi);
            }

            const value = extractEntity(step.result);

            if (value === null) {
              await step.context.sendActivity("Por favor, digite um e-mail válido.");
              return await step.replaceDialog('/hear', step.activeDialog.state.options);
            }

            result = value;

          }
          else if (step.activeDialog.state.options['kind'] === "name") {
            const extractEntity = text => {
              return text.match(/[_a-zA-Z][_a-zA-Z0-9]{0,16}/gi);
            };

            const value = extractEntity(step.result);

            if (value === null || value.length != 1) {
              await step.context.sendActivity("Por favor, digite um nome válido.");
              return await step.replaceDialog('/hear', step.activeDialog.state.options);
            }

            result = value;

          }
          else if (step.activeDialog.state.options['kind'] === "integer") {
            const extractEntity = text => {
              return text.match(/\d+/gi);
            };

            const value = extractEntity(step.result);

            if (value === null || value.length != 1) {
              await step.context.sendActivity("Por favor, digite um número válido.");
              return await step.replaceDialog('/hear', step.activeDialog.state.options);
            }

            result = value;
          }
          else if (step.activeDialog.state.options['kind'] === "date") {
            const extractEntity = text => {
              return text.match(/(^(((0[1-9]|1[0-9]|2[0-8])[\/](0[1-9]|1[012]))|((29|30|31)[\/](0[13578]|1[02]))|((29|30)[\/](0[4,6,9]|11)))[\/](19|[2-9][0-9])\d\d$)|(^29[\/]02[\/](19|[2-9][0-9])(00|04|08|12|16|20|24|28|32|36|40|44|48|52|56|60|64|68|72|76|80|84|88|92|96)$)/gi);
            };

            const value = extractEntity(step.result);

            if (value === null || value.length != 1) {
              await step.context.sendActivity("Por favor, digite uma data no formato 12/12/2020.");
              return await step.replaceDialog('/hear', step.activeDialog.state.options);
            }

            result = value;
          }
          else if (step.activeDialog.state.options['kind'] === "hour") {

            const extractEntity = text => {
              return text.match(/^([0-1]?[0-9]|2[0-4]):([0-5][0-9])(:[0-5][0-9])?$/gi);
            };

            const value = extractEntity(step.result);

            if (value === null || value.length != 1) {
              await step.context.sendActivity("Por favor, digite um horário no formato hh:ss.");
              return await step.replaceDialog('/hear', step.activeDialog.state.options);
            }

            result = value;
          }
          else if (step.activeDialog.state.options['kind'] === "money") {
            const extractEntity = text => {

              if (step.context.locale === 'en') { // TODO: Change to user.
                return text.match(/(?:\d{1,3},)*\d{1,3}(?:\.\d+)?/gi);
              }
              else {
                return text.match(/(?:\d{1,3}.)*\d{1,3}(?:\,\d+)?/gi);
              }
            };

            const value = extractEntity(step.result);

            if (value === null || value.length != 1) {
              await step.context.sendActivity("Por favor, digite um valor monetário.");
              return await step.replaceDialog('/hear', step.activeDialog.state.options);
            }

            result = value;
          }
          else if (step.activeDialog.state.options['kind'] === "mobile") {
            const locale = step.context.activity.locale;
            let phoneNumber;
            try {
              phoneNumber = phone(step.result, 'BRA')[0]; // TODO: Use accordingly to the person.
              phoneNumber = phoneUtil.parse(phoneNumber);
            } catch (error) {
              await step.context.sendActivity(Messages[locale].validation_enter_valid_mobile);

              return await step.replaceDialog('/profile_mobile', step.activeDialog.state.options);
            }
            if (!phoneUtil.isPossibleNumber(phoneNumber)) {
              await step.context.sendActivity("Por favor, digite um número de telefone válido.");
              return await step.replaceDialog('/hear', step.activeDialog.state.options);
            }

            result = phoneNumber;

          }
          else if (step.activeDialog.state.options['kind'] === "zipcode") {
            const extractEntity = text => {

              text = text.replace(/\-/gi, '');

              if (step.context.locale === 'en') { // TODO: Change to user.
                return text.match(/\d{8}/gi);
              }
              else {
                return text.match(/(?:\d{1,3}.)*\d{1,3}(?:\,\d+)?/gi);

              }
            };

            const value = extractEntity(step.result);

            if (value === null || value.length != 1) {
              await step.context.sendActivity("Por favor, digite um valor monetário.");
              return await step.replaceDialog('/hear', step.activeDialog.state.options);
            }

            result = value[0];

          }
          else if (step.activeDialog.state.options['kind'] === "menu") {

            const list = step.activeDialog.state.options['args'];
            result = null;
            await CollectionUtil.asyncForEach(list, async item => {
              if (GBConversationalService.kmpSearch(step.result, item) != -1) {
                result = item;
              }
            });

            if (result === null) {
              await step.context.sendActivity(`Escolha por favor um dos itens sugeridos.`);
              return await step.replaceDialog('/hear', step.activeDialog.state.options);
            }
          }
          else if (step.activeDialog.state.options['kind'] === "language") {

            result = null;

            const list = [
              { name: 'english', code: 'en' },
              { name: 'inglês', code: 'en' },
              { name: 'portuguese', code: 'pt' },
              { name: 'português', code: 'pt' },
              { name: 'français', code: 'fr' },
              { name: 'francês', code: 'fr' },
              { name: 'french', code: 'fr' },
              { name: 'spanish', code: 'es' },
              { name: 'espanõl', code: 'es' },
              { name: 'espanhol', code: 'es' },
              { name: 'german', code: 'de' },
              { name: 'deutsch', code: 'de' },
              { name: 'alemão', code: 'de' }
            ];

            const text = step.context.activity['originalText'];

            await CollectionUtil.asyncForEach(list, async item => {
              if (GBConversationalService.kmpSearch(text.toLowerCase(), item.name.toLowerCase()) != -1 ||
                GBConversationalService.kmpSearch(text.toLowerCase(), item.code.toLowerCase()) != -1) {
                result = item.code;
              }
            });

            if (result === null) {
              await min.conversationalService.sendText(min, step, `Escolha por favor um dos idiomas sugeridos.`);
              return await step.replaceDialog('/hear', step.activeDialog.state.options);
            }

          }

          const id = step.activeDialog.state.options.id;
          if (min.cbMap[id]) {
            const promise = min.cbMap[id].promise;
            delete min.cbMap[id];
            try {

              await promise(step, result);
              if (step.activeDialog.state.options.previousResolve != undefined) {
                step.activeDialog.state.options.previousResolve();
              }

              return await step.next();
            } catch (error) {
              GBLog.error(`Error in BASIC code: ${error}`);
              const locale = step.context.activity.locale;
              await min.conversationalService.sendText(min, step, Messages[locale].very_sorry_about_error);
            }
          }

        }
      ])
    );
  }

  public static async callVM(text: string, min: GBMinInstance, step: GBDialogStep, deployer: GBDeployer) {
    const sandbox: DialogKeywords = new DialogKeywords(min, deployer);
    const context = vm.createContext(sandbox);
    const code = min.sandBoxMap[text];
    vm.runInContext(code, context);

    const mainMethod = text.toLowerCase();
    sandbox[mainMethod].bind(sandbox);

    let ret = null;
    try {
      ret = await sandbox[mainMethod](step);

    } catch (error) {
      GBLog.error(`BASIC ERROR: ${error.message} ${error.stack}`);
    }

    return ret;
  }
}
