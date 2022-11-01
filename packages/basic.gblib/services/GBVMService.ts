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

import { GBLog, GBMinInstance, GBService, IGBCoreService, GBDialogStep } from 'botlib';
import * as fs from 'fs';
import { GBDeployer } from '../../core.gbapp/services/GBDeployer';
import { TSCompiler } from './TSCompiler';
import { CollectionUtil } from 'pragmatismo-io-framework';
const urlJoin = require('url-join');
import { DialogKeywords } from './DialogKeywords';
import { ScheduleServices } from './ScheduleServices';
import { GBConfigService } from '../../core.gbapp/services/GBConfigService';
//tslint:disable-next-line:no-submodule-imports
const { NodeVM, VMScript } = require('vm2');
const { createVm2Pool } = require('./vm2-process/index');

const vb2ts = require('./vbscript-to-typescript');
const beautify = require('js-beautify').js;
const textract = require('textract');
const walkPromise = require('walk-promise');

const Path = require('path');
/**
 * @fileoverview Virtualization services for emulation of BASIC.
 * This alpha version is using a antipattern hack in form of converter to
 * translate BASIC to TS and string replacements to emulate await code.
 * See https://github.com/uweg/vbscript-to-typescript for more info on vb2ts, so
 * http://stevehanov.ca/blog/index.php?id=92 should be used to run it without
 * translation and enhance classic BASIC experience.
 */

/**
 * Basic services for BASIC manipulation.
 */
export class GBVMService extends GBService {
  public async loadDialogPackage(folder: string, min: GBMinInstance, core: IGBCoreService, deployer: GBDeployer) {
    const files = await walkPromise(folder);

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
        const interval = 3000; // If compiled is older 30 seconds, then recompile.
        let writeVBS = true;
        if (fs.existsSync(fullVbsFile)) {
          const vbsStat = fs.statSync(fullVbsFile);
          if (docxStat['mtimeMs'] < vbsStat['mtimeMs'] + interval) {
            writeVBS = false;
          }
        }
        filename = vbsFile;
        let mainName = GBVMService.getMethodNameFromVBSFilename(filename);
        min.scriptMap[filename] = mainName;

        if (writeVBS) {
          let text = await this.getTextFromWord(folder, wordFile);

          const schedule = GBVMService.getSetScheduleKeywordArgs(text);
          const s = new ScheduleServices();
          if (schedule) {
            await s.createOrUpdateSchedule(min, schedule, mainName);
          }
          else {
            await s.deleteScheduleIfAny(min, mainName);
          }
          text = text.replace(/SET SCHEDULE (.*)/gi, '');
          fs.writeFileSync(urlJoin(folder, vbsFile), text);
        }

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
          if (compiledAt.isFile() && compiledAt['mtimeMs'] > jsStat['mtimeMs'] + interval) {
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

  public static getSetScheduleKeywordArgs(code: string) {
    if (!code)
      return null;
    const keyword = /SET SCHEDULE (.*)/gi;
    const result = keyword.exec(code);
    return result ? result[1] : null;
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
    step=dk.step
    id = sys().getRandomId()
    username = step ? dk.userName(step) : sys().getRandomId();
    mobile = step ? dk.userMobile(step) : sys().getRandomId();
    from = mobile;
    ENTER = String.fromCharCode(13);
    ubound = function(array){return array.length};
    isarray = function(array){return Array.isArray(array) };
    weekday = dk.getWeekFromDate.bind(dk);
    hour = dk.getHourFromDate.bind(dk);
    base64 = dk.getCoded;
    tolist = dk.getToLst;
    headers = {};
    data = {};
    list = [];
    httpUsername = "";
    httpPs = "";

    ${process.env.ENABLE_AUTH ? `hear gbLogin as login` : ``}

    ${code}


    
    `;

    // Keywords from General Bots BASIC.

    code = code.replace(/(\w+)\s*\=\s*SELECT\s*(.*)/gi, ($0, $1, $2) => {

      let tableName = /\sFROM\s(\w+)/.exec($2)[1];
      let sql = `SELECT ${$2}`.replace(tableName, '?');
      return `${$1} = sys().executeSQL(${$1}, "${sql}", "${tableName}")\n`;
    });


    code = code.replace(/(\w+)\s*\=\s*get html\s*(.*)/gi, ($0, $1, $2, $3) => {
      return `${$1} = getPage(step, ${$2})\n`;
    });
    code = code.replace(/(set hear on)(\s*)(.*)/gi, ($0, $1, $2, $3) => {
      return `hrOn = ${$3}\n`;
    });

    code = code.replace(/hear (\w+) as login/gi, ($0, $1) => {
      return `${$1} = hear("login")`;
    });
    code = code.replace(/hear (\w+) as email/gi, ($0, $1) => {
      return `${$1} = hear("email")`;
    });

    code = code.replace(/hear (\w+) as integer/gi, ($0, $1, $2) => {
      return `${$1} = hear("integer")`;
    });

    code = code.replace(/hear (\w+) as file/gi, ($0, $1, $2) => {
      return `${$1} = hear("file")`;
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

    code = code.replace(/(hear)\s*(\w+)/gi, ($0, $1, $2) => {
      return `${$2} = hear()`;
    });

    code = code.replace(/(\w)\s*\=\s*find contact\s*(.*)/gi, ($0, $1, $2, $3) => {
      return `${$1} = fndContact(${$2})\n`;
    });

    code = code.replace(/(\w+)\s*=\s*find\s*(.*)\s*or talk\s*(.*)/gi, ($0, $1, $2, $3) => {
      return `${$1} = sys().find(${$2})\n
      if (!${$1}) {
        if (resolve){
          resolve();
        }
        talk (${$3})\n;
        return -1;
      }
      `;
    });

    code = code.replace(/CALL\s*(.*)/gi, ($0, $1, $2, $3) => {
      return `sys().callVM("${$1}", dk.getMin(), dk.getStep(), dk.getDeployer())\n`;
    });

    code = code.replace(/(\w)\s*\=\s*find\s*(.*)/gi, ($0, $1, $2, $3) => {
      return `${$1} = sys().find(${$2})\n`;
    });

    code = code.replace(/(\w)\s*\=\s*create deal(\s)(.*)/gi, ($0, $1, $2, $3) => {
      return `${$1} =createDeal(${$3})\n`;
    });

    code = code.replace(/(\w)\s*\=\s*active tasks/gi, ($0, $1) => {
      return `${$1} = getActiveTasks()\n`;
    });

    code = code.replace(/(\w)\s*\=\s*append\s*(.*)/gi, ($0, $1, $2, $3) => {
      return `${$1} = sys().append(${$2})\n`;
    });

    code = code.replace(/(\w+)\s*\=\s*sort\s*(\w+)\s*by(.*)/gi, ($0, $1, $2, $3) => {
      return `${$1} = sys().sortBy(${$2}, "${$3}")\n`;
    });

    code = code.replace(/see\s*text\s*of\s*(\w+)\s*as\s*(\w+)\s*/gi, ($0, $1, $2, $3) => {
      return `${$2} = sys().seeText(${$1})\n`;
    });

    code = code.replace(/see\s*caption\s*of\s*(\w+)\s*as(.*)/gi, ($0, $1, $2, $3) => {
      return `${$2} = sys().seeCaption(${$1})\n`;
    });

    code = code.replace(/(wait)\s*(\d+)/gi, ($0, $1, $2) => {
      return `sys().wait(${$2})`;
    });

    code = code.replace(/(get stock for )(.*)/gi, ($0, $1, $2) => {
      return `stock = sys().getStock(${$2})`;
    });

    code = code.replace(/(\w+)\s*\=\s*get\s(.*)/gi, ($0, $1, $2, $3) => {

      const count = ($2.match(/\,/g) || []).length;
      const values = $2.split(',');

      // Handles GET page, "selector".

      if (count == 1) {

        return `${$1} = this.getBySelector(${values[0]}, ${values[1]} )`;
      }

      // Handles GET page, "frameSelector", "selector"

      else if (count == 2) {

        return `${$1} = this.getByFrame(${values[0]}, ${values[1]}, ${values[2]} )`;
      }

      // Handles the GET http version.

      else {

        return `${$1} = sys().get (${$2}, headers, httpUsername, httpPs)`;
      }

    });

    code = code.replace(/\= NEW OBJECT/gi, ($0, $1, $2, $3) => {
      return ` = {}`;
    });

    code = code.replace(/\= NEW ARRAY/gi, ($0, $1, $2, $3) => {
      return ` = []`;
    });


    code = code.replace(/(go to)(\s)(.*)/gi, ($0, $1, $2, $3) => {
      return `gotoDialog(step, ${$3})\n`;
    });

    code = code.replace(/(set language)(\s*)(.*)/gi, ($0, $1, $2, $3) => {
      return `setLanguage (step, ${$3})\n`;
    });

    code = code.replace(/set header\s*(.*)\sas\s(.*)/gi, ($0, $1, $2) => {
      return `headers[${$1}]=${$2})`;
    });

    code = code.replace(/set http username\s*\=\s*(.*)/gi, ($0, $1) => {
      return `httpUsername = ${$1}`;
    });

    code = code.replace(/set http password\s*\=\s*(.*)/gi, ($0, $1) => {
      return `httpPs = ${$1}`;
    });

    code = code.replace(/(datediff)(\s*)(.*)/gi, ($0, $1, $2, $3) => {
      return `dateDiff (step, ${$3})\n`;
    });

    code = code.replace(/(dateadd)(\s*)(.*)/gi, ($0, $1, $2, $3) => {
      return `dateAdd (step, ${$3})\n`;
    });

    code = code.replace(/(set max lines)(\s*)(.*)/gi, ($0, $1, $2, $3) => {
      return `setMaxLines (step, ${$3})\n`;
    });

    code = code.replace(/(set max columns)(\s*)(.*)/gi, ($0, $1, $2, $3) => {
      return `setMaxColumns (step, ${$3})\n`;
    });

    code = code.replace(/(set translator)(\s*)(.*)/gi, ($0, $1, $2, $3) => {
      return `setTranslatorOn (step, "${$3.toLowerCase()}")\n`;
    });

    code = code.replace(/(set theme)(\s*)(.*)/gi, ($0, $1, $2, $3) => {
      return `setTheme (step, "${$3.toLowerCase()}")\n`;
    });

    code = code.replace(/(set whole word)(\s*)(.*)/gi, ($0, $1, $2, $3) => {
      return `setWholeWord (step, "${$3.toLowerCase()}")\n`;
    });

    code = code.replace(/(\w+)\s*\=\s*post\s*(.*),\s*(.*)/gi, ($0, $1, $2, $3) => {
      return `${$1} = sys().postByHttp (${$2}, ${$3}, headers)`;
    });

    code = code.replace(/(\w+)\s*\=\s*put\s*(.*),\s*(.*)/gi, ($0, $1, $2, $3) => {
      return `${$1} = sys().putByHttp (${$2}, ${$3}, headers)`;
    });

    code = code.replace(/(\w+)\s*\=\s*download\s*(.*),\s*(.*)/gi, ($0, $1, $2, $3) => {
      return `${$1} = sys().download (${$2}, ${$3})`;
    });

    code = code.replace(/(\w+)\s*\=\s*CREATE FOLDER\s*(.*)/gi, ($0, $1, $2) => {
      return `${$1} = sys().createFolder (${$2})`;
    });

    code = code.replace(/SHARE FOLDER\s*(.*)/gi, ($0, $1) => {
      return `sys().shareFolder (${$1})`;
    });

    code = code.replace(/(create a bot farm using)(\s)(.*)/gi, ($0, $1, $2, $3) => {
      return `sys().createABotFarmUsing (${$3})`;
    });

    code = code.replace(/(chart)(\s)(.*)/gi, ($0, $1, $2, $3) => {
      return `chart (step, ${$3})\n`;
    });

    code = code.replace(/(transfer to)(\s)(.*)/gi, ($0, $1, $2, $3) => {
      return `transferTo (step, ${$3})\n`;
    });

    code = code.replace(/(\btransfer\b)(?=(?:[^"]|"[^"]*")*$)/gi, () => {
      return `transferTo (step)\n`;
    });

    code = code.replace(/(exit)/gi, () => {
      return `if(resolve) {resolve();}\n`;
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

    code = code.replace(/(send email)(\s*)(.*)/gi, ($0, $1, $2, $3) => {
      return `sendEmail (${$3})\n`;
    });

    code = code.replace(/(send mail)(\s*)(.*)/gi, ($0, $1, $2, $3) => {
      return `sendEmail (${$3})\n`;
    });

    code = code.replace(/(send file to)(\s*)(.*)/gi, ($0, $1, $2, $3) => {
      return `sendFileTo (step, ${$3})\n`;
    });

    code = code.replace(/(hover)(\s*)(.*)/gi, ($0, $1, $2, $3) => {
      return `hover (step, ${$3})\n`;
    });

    code = code.replace(/(click link text)(\s*)(.*)/gi, ($0, $1, $2, $3) => {
      return `linkByText (step, ${$3})\n`;
    });

    code = code.replace(/(click)(\s*)(.*)/gi, ($0, $1, $2, $3) => {
      return `click (step, ${$3})\n`;
    });

    code = code.replace(/(send file)(\s*)(.*)/gi, ($0, $1, $2, $3) => {
      return `sendFile (step, ${$3})\n`;
    });

    code = code.replace(/(copy)(\s*)(.*)/gi, ($0, $1, $2, $3) => {
      return `sys().copyFile(${$3})\n`;
    });

    code = code.replace(/(convert)(\s*)(.*)/gi, ($0, $1, $2, $3) => {
      return `sys().convert(${$3})\n`;
    });

    // TODO: AS CHART.
    // code = code.replace(/(\w+)\s*\=\s*(.*)\s*as chart/gi, ($0, $1, $2) => {
    //   return `${$1} = sys().asImage(${$2})\n`;
    // });

    code = code.replace(/MERGE\s(.*)\sWITH\s(.*)BY\s(.*)/gi, ($0, $1, $2, $3) => {
      return `sys().merge(${$1}, ${$2}, ${$3})\n`;
    });

    code = code.replace(/PRESS\s(.*)\sON\s(.*)/gi, ($0, $1, $2) => {
      return `pressKey(step, ${$2}, ${$1})\n`;
    });

    code = code.replace(/SCREENSHOT\s(.*)/gi, ($0, $1, $2) => {
      return `screenshot(step, ${$1})\n`;
    });

    code = code.replace(/TWEET\s(.*)/gi, ($0, $1, $2) => {
      return `sys().tweet(step, ${$1})\n`;
    });

    code = code.replace(/(\w+)\s*\=\s*(.*)\s*as image/gi, ($0, $1, $2) => {
      return `${$1} = sys().asImage(${$2})\n`;
    });

    code = code.replace(/(\w+)\s*\=\s*(.*)\s*as pdf/gi, ($0, $1, $2) => {
      return `${$1} = sys().asPdf(${$2})\n`;
    });

    code = code.replace(/(\w+)\s*\=\s*FILL\s(.*)\sWITH\s(.*)/gi, ($0, $1, $2, $3) => {
      return `${1} = sys().fill(${$2}, ${$3})\n`;
    });

    code = code.replace(/save\s(.*)\sas\s(.*)/gi, ($0, $1, $2, $3) => {
      return `sys().saveFile(${$2}, ${$1})\n`;
    });
    code = code.replace(/(save)(\s)(.*)/gi, ($0, $1, $2, $3) => {
      return `sys().save(${$3})\n`;
    });

    code = code.replace(/set\s(.*)/gi, ($0, $1, $2) => {
      return `sys().set (${$1})`;
    });


    code = `${code}\n%>`;

    return code;
  }

  public async executeBASIC(filename: any, min: GBMinInstance, deployer: GBDeployer, mainName: string) {

    // Converts General Bots BASIC into regular VBS

    let basicCode: string = fs.readFileSync(filename, 'utf8');

    // Processes END keyword, removing extracode, useful
    // for development.

    let end = /(\nend\n)/gi.exec(basicCode);
    if (end) {
      basicCode = basicCode.substring(0, end.index);
    }

    // Removes comments.

    basicCode = basicCode.replace(/((^|\W)REM.*\n)/gi, '');

    // Process INCLUDE keyword to include another
    // dialog inside the dialog.

    let include = null;
    do {
      include = /^include\b(.*)$/gmi.exec(basicCode);

      if (include) {
        let includeName = include[1].trim();
        includeName = Path.join(Path.dirname(filename), includeName);
        includeName = includeName.substr(0, includeName.lastIndexOf(".")) + ".vbs";

        // To use include, two /publish will be necessary (for now)
        // because of alphabet order may raise not found errors.

        let includeCode: string = fs.readFileSync(includeName, 'utf8');
        basicCode = basicCode.replace(/^include\b.*$/gmi, includeCode);
      }
    } while (include);

    const vbsCode = this.convertGBASICToVBS(basicCode);
    const vbsFile = `${filename}.compiled`;
    fs.writeFileSync(vbsFile, vbsCode);

    // Converts VBS into TS.

    vb2ts.convertFile(vbsFile);

    // Convert TS into JS.

    const tsfile: string = `${filename}.ts`;
    let tsCode: string = fs.readFileSync(tsfile, 'utf8');
    tsCode = tsCode + `let resolve;`;
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

      parsedCode = this.handleThisAndAwait(parsedCode);

      parsedCode = parsedCode.replace(/(\bnow\b)(?=(?:[^"]|"[^"]*")*$)/gi, 'await dk.getNow()');
      parsedCode = parsedCode.replace(/(\btoday\b)(?=(?:[^"]|"[^"]*")*$)/gi, 'await dk.getToday(step)');
      parsedCode = parsedCode.replace(/(\bweekday\b)(?=(?:[^"]|"[^"]*")*$)/gi, 'weekday');
      parsedCode = parsedCode.replace(/(\bhour\b)(?=(?:[^"]|"[^"]*")*$)/gi, 'hour');
      parsedCode = parsedCode.replace(/(\btolist\b)(?=(?:[^"]|"[^"]*")*$)/gi, 'tolist');

      parsedCode = beautify(parsedCode, { indent_size: 2, space_in_empty_paren: true });
      fs.writeFileSync(jsfile, parsedCode);

      this.executeJS(min, deployer, parsedCode, mainName);
      GBLog.info(`[GBVMService] Finished loading of ${filename}, JavaScript from Word: \n ${parsedCode}`);
    }
  }

  private executeJS(min: GBMinInstance, deployer: GBDeployer, parsedCode: string, mainName: string) {
    try {
      min.sandBoxMap[mainName.toLowerCase().trim()] = parsedCode;
    } catch (error) {
      GBLog.error(`[GBVMService] ERROR loading ${error}`);
    }
  }

  private handleThisAndAwait(code: string) {
    // this insertion.

    code = code.replace(/sys\(\)/gi, 'dk.sys()');
    code = code.replace(/("[^"]*"|'[^']*')|\btalk\b/gi, ($0, $1) => {
      return $1 === undefined ? 'dk.talk' : $1;
    });
    code = code.replace(/("[^"]*"|'[^']*')|\bhear\b/gi, ($0, $1) => {
      return $1 === undefined ? 'dk.hear' : $1;
    });
    code = code.replace(/("[^"]*"|'[^']*')|\baskEmail\b/gi, ($0, $1) => {
      return $1 === undefined ? 'dk.askEmail' : $1;
    });
    code = code.replace(/("[^"]*"|'[^']*')|\bsendFileTo\b/gi, ($0, $1) => {
      return $1 === undefined ? 'dk.sendFileTo' : $1;
    });
    code = code.replace(/("[^"]*"|'[^']*')|\bsendFile\b/gi, ($0, $1) => {
      return $1 === undefined ? 'dk.sendFile' : $1;
    });
    code = code.replace(/("[^"]*"|'[^']*')|\bsetLanguage\b/gi, ($0, $1) => {
      return $1 === undefined ? 'dk.setLanguage' : $1;
    });
    code = code.replace(/("[^"]*"|'[^']*')|\bdateAdd\b/gi, ($0, $1) => {
      return $1 === undefined ? 'dk.dateAdd' : $1;
    });
    code = code.replace(/("[^"]*"|'[^']*')|\bdateDiff\b/gi, ($0, $1) => {
      return $1 === undefined ? 'dk.dateDiff' : $1;
    });
    code = code.replace(/("[^"]*"|'[^']*')|\bgotoDialog\b/gi, ($0, $1) => {
      return $1 === undefined ? 'dk.gotoDialog' : $1;
    });
    code = code.replace(/("[^"]*"|'[^']*')|\bsetMaxLines\b/gi, ($0, $1) => {
      return $1 === undefined ? 'dk.setMaxLines' : $1;
    });
    code = code.replace(/("[^"]*"|'[^']*')|\bsetTranslatorOn\b/gi, ($0, $1) => {
      return $1 === undefined ? 'dk.setTranslatorOn' : $1;
    });
    code = code.replace(/("[^"]*"|'[^']*')|\bsetTheme\b/gi, ($0, $1) => {
      return $1 === undefined ? 'dk.setTheme' : $1;
    });

    code = code.replace(/("[^"]*"|'[^']*')|\bsetWholeWord\b/gi, ($0, $1) => {
      return $1 === undefined ? 'dk.setWholeWord' : $1;
    });
    code = code.replace(/("[^"]*"|'[^']*')|\btransferTo\b/gi, ($0, $1) => {
      return $1 === undefined ? 'dk.transferTo' : $1;
    });
    code = code.replace(/("[^"]*"|'[^']*')|\bchart\b/gi, ($0, $1) => {
      return $1 === undefined ? 'dk.chart' : $1;
    });
    code = code.replace(/("[^"]*"|'[^']*')|\bcreateDeal\b/gi, ($0, $1) => {
      return $1 === undefined ? 'dk.createDeal' : $1;
    });
    code = code.replace(/("[^"]*"|'[^']*')|\bfndContact\b/gi, ($0, $1) => {
      return $1 === undefined ? 'dk.fndContact' : $1;
    });
    code = code.replace(/("[^"]*"|'[^']*')|\bgetActiveTasks\b/gi, ($0, $1) => {
      return $1 === undefined ? 'dk.getActiveTasks' : $1;
    });
    code = code.replace(/("[^"]*"|'[^']*')|\bmenu\b/gi, ($0, $1) => {
      return $1 === undefined ? 'dk.menu' : $1;
    });
    code = code.replace(/("[^"]*"|'[^']*')|\bgetPage\b/gi, ($0, $1) => {
      return $1 === undefined ? 'dk.getPage' : $1;
    });
    code = code.replace(/("[^"]*"|'[^']*')|\bclick\b/gi, ($0, $1) => {
      return $1 === undefined ? 'dk.click' : $1;
    });
    code = code.replace(/("[^"]*"|'[^']*')|\blinkByText\b/gi, ($0, $1) => {
      return $1 === undefined ? 'dk.linkByText' : $1;
    });
    code = code.replace(/("[^"]*"|'[^']*')|\bpressKey\b/gi, ($0, $1) => {
      return $1 === undefined ? 'dk.pressKey' : $1;
    });
    code = code.replace(/("[^"]*"|'[^']*')|\bscreenshot\b/gi, ($0, $1) => {
      return $1 === undefined ? 'dk.screenshot' : $1;
    });
    code = code.replace(/("[^"]*"|'[^']*')|\bhover\b/gi, ($0, $1) => {
      return $1 === undefined ? 'dk.hover' : $1;
    });
    code = code.replace(/("[^"]*"|'[^']*')|\bsendEmail\b/gi, ($0, $1) => {
      return $1 === undefined ? 'dk.sendEmail' : $1;
    });
    // await insertion.

    code = code.replace(/dk\./gm, 'await dk.');
    code = code.replace(/\nfunction/i, 'async function');
    code = code.replace('ubound = async', 'ubound =');  // TODO: Improve this.
    code = code.replace('hour = await', 'hour =');  // TODO: Improve this.
    code = code.replace('weekday = await', 'weekday =');  // TODO: Improve this.
    code = code.replace('tolist = await', 'tolist =');  // TODO: Improve this.
    code = code.replace('isarray = async', 'isarray =');  // TODO: Waiting for a compiler.
    code = code.replace('isArray = async', 'isarray =');  // TODO: Waiting for a compiler.
    code = code.replace('base64 = await', 'base64 =');  // TODO: Waiting for a compiler.

    return code;
  }

  /**
   * Executes the converted JavaScript from BASIC code inside execution context.
   */
  public static async callVM(text: string, min: GBMinInstance, step: GBDialogStep, deployer: GBDeployer) {

    // Creates a class DialogKeywords which is the *this* pointer
    // in BASIC.

    const user = step ? await min.userProfile.get(step.context, {}) : null;

    const sandbox: DialogKeywords = new DialogKeywords(min, deployer, user.systemUser);

    const contentLocale = min.core.getParam<string>(
      min.instance,
      'Default Content Language',
      GBConfigService.get('DEFAULT_CONTENT_LANGUAGE')
    );

    // Auto-NLP generates BASIC variables related to entities.

    if (step && step.context.activity['originalText']) {
      const entities = await min["nerEngine"].findEntities(
        step.context.activity['originalText'],
        contentLocale);

      for (let i = 0; i < entities.length; i++) {
        const v = entities[i];
        const variableName = `${v.entity}`;
        sandbox[variableName] = v.option;
      }
    }

    const botId = min.botId;
    const gbdialogPath = urlJoin(process.cwd(), 'work', `${botId}.gbai`, `${botId}.gbdialog`);
    const scriptPath = urlJoin(gbdialogPath, `${text}.js`);

    let code = min.sandBoxMap[text];

    if (GBConfigService.get('VM3')==='true'){
      try {

        const vm1 = new NodeVM({
          allowAsync: true,
          sandbox: {},
          console: 'inherit',
          wrapper: 'commonjs',
          require: {
            builtin: ['stream', 'http', 'https', 'url', 'zlib'],
            root: ['./'],
            external: true,
            context: 'sandbox'
          },
        });
        const s = new VMScript(code, { filename: scriptPath });
        let x = vm1.run(s);
        return x;
      } catch (error) {
        throw new Error(`BASIC RUNTIME ERR: ${error.message ? error.message : error}\n Stack:${error.stack}`);
      }

    } else {

      // {
      //   "name": "dev-rodriguez.gbdialog",
      //   "version": "1.0.0",
      //   "description": "",
      //    "author": "",
      //   "license": "ISC",
      //   "dependencies": {
      //     "encoding": "^0.1.13",
      //     "isomorphic-fetch": "^3.0.0",
      //     "punycode": "^2.1.1",
      //     "typescript-rest-rpc": "^1.0.10",
      //     "vm2": "^3.9.11"
      //   }
      // }

      const runnerPath = urlJoin(process.cwd(), 'dist', 'packages', 'basic.gblib', 'services', 'vm2-process', 'vm2ProcessRunner.js');

      try {
        const { run, drain } = createVm2Pool({
          min: 1,
          max: 1,
          cpu: 100,
          memory: 120000,
          time: 5520000,
          cwd: gbdialogPath,
          script: runnerPath
        });

        const result = await run(code, { filename: scriptPath });

        drain();
      } catch (error) {
        throw new Error(`BASIC RUNTIME ERR: ${error.message ? error.message : error}\n Stack:${error.stack}`);
      }
    }
  }
}
