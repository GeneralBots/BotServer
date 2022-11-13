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
import { ScheduleServices } from './ScheduleServices';
import { GBConfigService } from '../../core.gbapp/services/GBConfigService';
//tslint:disable-next-line:no-submodule-imports
const urlJoin = require('url-join');
const { NodeVM, VMScript } = require('vm2');
const { createVm2Pool } = require('./vm2-process/index');
const vb2ts = require('./vbscript-to-typescript');
const beautify = require('js-beautify').js;
const textract = require('textract');
const walkPromise = require('walk-promise');
const child_process = require('child_process');
const Path = require('path');
const indent = require('indent.js');
/**
 * @fileoverview  Decision was to priorize security(isolation) and debugging, 
 * over a beautiful BASIC transpiler (to be done).
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
          text = text.replace(/^\s*SET SCHEDULE (.*)/gim, '');
          fs.writeFileSync(urlJoin(folder, vbsFile), text);
        }

        // Process node_modules install.

        const node_modules = urlJoin(folder, 'node_modules');
        if (!fs.existsSync(node_modules)) {
          const packageJson = `
            {
              "name": "${min.botId}.gbdialog",
              "version": "1.0.0",
              "description": "${min.botId} transpiled .gbdialog",
              "author": "${min.botId} owner.",
              "license": "ISC",
              "dependencies": {
                "encoding": "0.1.13",
                "isomorphic-fetch": "3.0.0",
                "punycode": "2.1.1",
                "typescript-rest-rpc": "1.0.10",
                "vm2": "3.9.11"
              }
            }`;
          fs.writeFileSync(urlJoin(folder, 'package.json'), packageJson);

          GBLog.info(`BASIC: Installing .gbdialog node_modules for ${min.botId}...`);
          const npmPath = urlJoin(process.env.PWD, 'node_modules', '.bin', 'npm');
          child_process.execSync(`${npmPath} install`, { cwd: folder });
        }

        // Hot swap for .vbs files.

        const fullFilename = urlJoin(folder, filename);
        if (process.env.GBDIALOG_HOTSWAP) {
          fs.watchFile(fullFilename, async () => {
            await this.translateBASIC(fullFilename, min, deployer, mainName);
          });
        }

        const compiledAt = fs.statSync(fullFilename);
        const jsfile = urlJoin(folder, `${filename}.js`);

        if (fs.existsSync(jsfile)) {
          const jsStat = fs.statSync(jsfile);
          const interval = 30000; // If compiled is older 30 seconds, then recompile.
          if (compiledAt.isFile() && compiledAt['mtimeMs'] > jsStat['mtimeMs'] + interval) {
            await this.translateBASIC(fullFilename, min, deployer, mainName);
          } else {
            const parsedCode: string = fs.readFileSync(jsfile, 'utf8');

            min.sandBoxMap[mainName.toLowerCase().trim()] = parsedCode;
          }
        } else {
          await this.translateBASIC(fullFilename, min, deployer, mainName);
        }
      }
    });
  }

  public async translateBASIC(filename: any, min: GBMinInstance, deployer: GBDeployer, mainName: string) {

    // Converts General Bots BASIC into regular VBS

    let basicCode: string = fs.readFileSync(filename, 'utf8');

    // Processes END keyword, removing extracode, useful
    // for development in .gbdialog. 

    if (process.env.GBDIALOG_NOEND === 'true') {
      basicCode = basicCode.replace(/^\s*END(\W|\n)/gim, '');
    }
    else {
      let end = /^\s*END(\W|\n)/gi.exec(basicCode);
      if (end) {
        basicCode = basicCode.substring(0, end.index);
      }
    }

    // Removes comments.

    basicCode = basicCode.replace(/^\s*REM.*/gim, '');
    basicCode = basicCode.replace(/^\s*\'.*/gim, '');

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

    const [vbsCode, jsonMap] = await this.convertGBASICToVBS(min, basicCode);
    const vbsFile = `${filename}.compiled`;
    const mapFile = `${filename}.map`;

    fs.writeFileSync(vbsFile, vbsCode);
    fs.writeFileSync(mapFile, JSON.stringify(jsonMap));

    // Converts VBS into TS.

    vb2ts.convertFile(vbsFile);

    // Convert TS into JS.

    const tsfile: string = `${filename}.ts`;
    let tsCode: string = fs.readFileSync(tsfile, 'utf8');
    fs.writeFileSync(tsfile, tsCode);
    const tsc = new TSCompiler();
    tsc.compile([tsfile]);

    // Run JS into the GB context.

    const jsfile = `${tsfile}.js`.replace('.ts', '');

    if (fs.existsSync(jsfile)) {
      let code: string = fs.readFileSync(jsfile, 'utf8');

      code.replace(/^.*exports.*$/gm, '');

      code = `
      return (async () => {
        require('isomorphic-fetch');
        const rest = require ('typescript-rest-rpc/lib/client');

        // Interprocess communication from local HTTP to the BotServer.

        const dk = rest.createClient('http://localhost:1111/api/v2/${min.botId}/dialog');
        const sys = rest.createClient('http://localhost:1111/api/v2/${min.botId}/system');
        const wa = rest.createClient('http://localhost:1111/api/v2/${min.botId}/webautomation');
                
        // Local variables.

        const gb = await dk.getSingleton();
        const id = gb.id;
        const username = gb.username;
        const mobile = gb.mobile;
        const from = gb.from;
        const ENTER = gb.ENTER;
        const headers = gb.headers;
        const data = gb.data;
        const list = gb.list;
        const httpUsername = gb.httpUsername;
        const httpPs = gb.httpPs;
        let page = null;

    
        // Local functions.

        const ubound = (array) => {return array.length};
        const isarray = (array) => {return Array.isArray(array) };
    
        // Remote functions.
        
        const weekday = (v) => { return (async () => { return await dk.getWeekFromDate({v}) })(); };
        const hour = (v) => { return (async () => { return await dk.getHourFromDate({v}) })(); };
        const base64 =  (v) => { return (async () => { return await dk.getCoded({v}) })(); };
        const tolist =  (v) => { return (async () => { return await dk.getToLst({v}) })(); };
        const now =  (v) => { return (async () => { return await dk.getNow({v}) })(); };
        const today =  (v) => { return (async () => { return await dk.getToday({v}) })(); };

        ${code}

      })(); 
    
  `;
      code = indent.js(code, {tabString: '\t'});
      fs.writeFileSync(jsfile, code);
      min.sandBoxMap[mainName.toLowerCase().trim()] = code;
      GBLog.info(`[GBVMService] Finished loading of ${filename}, JavaScript from Word: \n ${code}`);
    }
  }

  public static getMethodNameFromVBSFilename(filename: string) {
    let mainName = filename.replace(/\s|\-/gim, '').split('.')[0];
    return mainName.toLowerCase();
  }

  public static getSetScheduleKeywordArgs(code: string) {
    if (!code)
      return null;
    const keyword = /^\s*SET SCHEDULE (.*)/gim;
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

  private getParams = (text, names) => {

    let ret = {};
    const splitParamsButIgnoreCommasInDoublequotes = (str) => {
      return str.split(',').reduce((accum, curr) => {
        if (accum.isConcatting) {
          accum.soFar[accum.soFar.length - 1] += ',' + curr
        } else {
          accum.soFar.push(curr)
        }
        if (curr.split('"').length % 2 == 0) {
          accum.isConcatting = !accum.isConcatting
        }
        return accum;
      }, { soFar: [], isConcatting: false }).soFar
    }

    const items = splitParamsButIgnoreCommasInDoublequotes(text);

    let i = 0;
    let json = '{';
    names.forEach(name => {
      let value = items[i];
      i++;
      json = `${json} "${name}": ${value} ${names.length == i ? '' : ','}`;
    });
    json = `${json}}`

    return json;
  };


  /**
   * Converts General Bots BASIC
   *
   *
   * @param code General Bots BASIC
   */
  public async convertGBASICToVBS(min: GBMinInstance, code: string) {

    // Start and End of VB2TS tags of processing.

    code = `<%\n

    ${process.env.ENABLE_AUTH ? `hear gbLogin as login` : ``}

    ${code}

    `;

    var allLines = code.split("\n");
    const keywords = this.getKeywords();
    const offset = 37;
    const jsonMap = {};

    for (var i = 0; i < allLines.length; i++) {
      for (var j = 0; j < keywords.length; j++) {
        allLines[i] = allLines[i].replace(keywords[j][0], keywords[j][1]);

        //  Add additional lines returned from replacement.

        let add = allLines[i].split(/\r\n|\r|\n/).length;
        jsonMap[i] = (offset + i) + (add ? add : 0);
      }
    }

    code = `${allLines.join('\n')}\n%>`;
    return [code, jsonMap];
  }

  private getKeywords() {

    // Keywords from General Bots BASIC.

    let keywords = [];
    let i = 0;

    keywords[i++] = [/^\s*(\w+)\s*\=\s*SELECT\s*(.*)/gim, ($0, $1, $2) => {
      let tableName = /\sFROM\s(\w+)/.exec($2)[1];
      let sql = `SELECT ${$2}`.replace(tableName, '?');
      return `${$1} = await sys.executeSQL({data:${$1}, sql:"${sql}", tableName:"${tableName}"})\n`;
    }];

    keywords[i++] = [/^\s*open\s*(.*)/gim, ($0, $1, $2) => {

      if (!$1.startsWith("\"") && !$1.startsWith("\'")) {
        $1 = `"${$1}"`;
      }
      const params = this.getParams($1, ['url', 'username', 'password']);

      return `page = await wa.getPage(${params})\n`;
    }];

    keywords[i++] = [/^\s*(set hear on)(\s*)(.*)/gim, ($0, $1, $2, $3) => {
      return `hrOn = ${$3}\n`;
    }];

    keywords[i++] = [/^\shear (\w+) as login/gim, ($0, $1) => {
      return `${$1} = await dk.getHear({kind:"login"})`;
    }];

    keywords[i++] = [/^\shear (\w+) as email/gim, ($0, $1) => {
      return `${$1} = await dk.getHear({kind:"email"})`;
    }];

    keywords[i++] = [/^\shear (\w+) as integer/gim, ($0, $1) => {
      return `${$1} = await dk.getHear({kind:"integer"})`;
    }];

    keywords[i++] = [/^\shear (\w+) as file/gim, ($0, $1) => {
      return `${$1} = await dk.getHear({kind:"file"})`;
    }];

    keywords[i++] = [/^\shear (\w+) as boolean/gim, ($0, $1) => {
      return `${$1} = await dk.getHear({kind:"boolean"})`;
    }];

    keywords[i++] = [/^\shear (\w+) as name/gim, ($0, $1) => {
      return `${$1} = await dk.getHear({kind:"name"})`;
    }];

    keywords[i++] = [/^\shear (\w+) as date/gim, ($0, $1) => {
      return `${$1} = await dk.getHear({kind:"date"})`;
    }];

    keywords[i++] = [/^\shear (\w+) as hour/gim, ($0, $1) => {
      return `${$1} = await dk.getHear({kind:"hour"})`;
    }];

    keywords[i++] = [/^\shear (\w+) as phone/gim, ($0, $1) => {
      return `${$1} = await dk.getHear({kind:"phone"})`;
    }];

    keywords[i++] = [/^\shear (\w+) as money/gim, ($0, $1) => {
      return `${$1} = await dk.getHear({kind:"money")}`;
    }];

    keywords[i++] = [/^\shear (\w+) as language/gim, ($0, $1) => {
      return `${$1} = await dk.getHear({kind:"language")}`;
    }];

    keywords[i++] = [/^\shear (\w+) as zipcode/gim, ($0, $1) => {
      return `${$1} = await dk.getHear({kind:"zipcode")}`;
    }];

    keywords[i++] = [/^\shear (\w+) as (.*)/gim, ($0, $1, $2) => {
      return `${$1} = await dk.getHear({kind:"menu", args: [${$2}])}`;
    }];

    keywords[i++] = [/^\s*(hear)\s*(\w+)/gim, ($0, $1, $2) => {
      return `${$2} = await dk.getHear({})`;
    }];

    keywords[i++] = [/^\s*(\w+)\s*\=\s*find contact\s*(.*)/gim, ($0, $1, $2, $3) => {
      return `${$1} = await dk.fndContact({${$2})\n`;
    }];

    keywords[i++] = [/^\s*(\w+)\s*=\s*find\s*(.*)\s*or talk\s*(.*)/gim, ($0, $1, $2, $3) => {
      return `${$1} = await sys.find({args:[${$2}])\n
      if (!${$1}) {
        await dk.talk ({${$3}})\n;
        return -1;
      }
      `;
    }];

    keywords[i++] = [/^\sCALL\s*(.*)/gim, ($0, $1) => {
      return `await ${$1}\n`;
    }];

    keywords[i++] = [/^\s*(\w+)\s*\=\s*find\s*(.*)/gim, ($0, $1, $2, $3) => {
      return `
      ${$1} = await sys.find({args: [${$2}]})\n`;
    }];

    keywords[i++] = [/^\s*(\w+)\s*\=\s*create deal(\s)(.*)/gim, ($0, $1, $2, $3) => {
      const params = this.getParams($3, ['dealName', 'contact', 'company', 'amount']);

      return `${$1} = await dk.createDeal(${params})\n`;
    }];

    keywords[i++] = [/^\s*(\w+)\s*\=\s*active tasks/gim, ($0, $1) => {
      return `${$1} = await dk.getActiveTasks({})\n`;
    }];

    keywords[i++] = [/^\s*(\w+)\s*\=\s*append\s*(.*)/gim, ($0, $1, $2, $3) => {
      return `${$1} = await sys.append({args:[${$2}]})\n`;
    }];

    keywords[i++] = [/^\s*(\w+)\s*\=\s*sort\s*(\w+)\s*by(.*)/gim, ($0, $1, $2, $3) => {
      return `${$1} = await sys.sortBy({array: ${$2}, memberName: "${$3}"})\n`;
    }];

    keywords[i++] = [/^\ssee\s*text\s*of\s*(\w+)\s*as\s*(\w+)\s*/gim, ($0, $1, $2, $3) => {
      return `${$2} = await sys.seeText({url: ${$1})\n`;
    }];

    keywords[i++] = [/^\ssee\s*caption\s*of\s*(\w+)\s*as(.*)/gim, ($0, $1, $2, $3) => {
      return `${$2} = await sys.seeCaption({url: ${$1})\n`;
    }];

    keywords[i++] = [/^\s*(wait)\s*(\d+)/gim, ($0, $1, $2) => {
      return `await sys.wait({seconds:${$2}})`;
    }];

    keywords[i++] = [/^\s*(get stock for )(.*)/gim, ($0, $1, $2) => {
      return `stock = await sys.getStock({symbol: ${$2})`;
    }];

    keywords[i++] = [/^\s*(\w+)\s*\=\s*get\s(.*)/gim, ($0, $1, $2, $3) => {

      const count = ($2.match(/\,/g) || []).length;
      const values = $2.split(',');

      // Handles GET "selector".

      if (count == 1) {

        return `${$1} =  await wa.getBySelector({handle:page, selector: ${values[0]}})`;
      }

      // Handles GET "frameSelector", "selector"

      else if (count == 2) {

        return `${$1} =  await wa.getByFrame({handle: page, ${values[0]}, frameOrSelector: ${values[1]}, selector: ${values[2]}})`;
      }

      // Handles the GET http version.

      else {
        return `${$1} = await sys.get ({file: ${$2}, addressOrHeaders: headers, httpUsername, httpPs})`;
      }

    }];

    keywords[i++] = [/\= NEW OBJECT/gi, ($0, $1, $2, $3) => {
      return ` = {}`;
    }];

    keywords[i++] = [/\= NEW ARRAY/gi, ($0, $1, $2, $3) => {
      return ` = []`;
    }];


    keywords[i++] = [/^\s*(go to)(\s)(.*)/gim, ($0, $1, $2, $3) => {
      const params = this.getParams($3, ['fromOrDialogName', 'dialogName']);
      return `await dk.gotoDialog(${params})\n`;
    }];

    keywords[i++] = [/^\s*(set language)(\s*)(.*)/gim, ($0, $1, $2, $3) => {
      return `await dk.setLanguage ({${$3}})\n`;
    }];

    keywords[i++] = [/^\s*set header\s*(.*)\sas\s(.*)/gim, ($0, $1, $2) => {
      return `headers[${$1}]=${$2})`;
    }];

    keywords[i++] = [/^\s*set http username\s*\=\s*(.*)/gim, ($0, $1) => {
      return `httpUsername = ${$1}`;
    }];

    keywords[i++] = [/^\sset http password\s*\=\s*(.*)/gim, ($0, $1) => {
      return `httpPs = ${$1}`;
    }];

    keywords[i++] = [/^\s*(datediff)(\s*)(.*)/gim, ($0, $1, $2, $3) => {
      const params = this.getParams($3, ['date1', 'date2', 'mode']);
      return `await dk.dateDiff (${params}})\n`;
    }];

    keywords[i++] = [/^\s*(dateadd)(\s*)(.*)/gim, ($0, $1, $2, $3) => {
      const params = this.getParams($3, ['date', 'mode', 'units']);
      return `await dk.dateAdd (${$3})\n`;
    }];

    keywords[i++] = [/^\s*(set max lines)(\s*)(.*)/gim, ($0, $1, $2, $3) => {
      return `await dk.setMaxLines ({count: ${$3}})\n`;
    }];

    keywords[i++] = [/^\s*(set max columns)(\s*)(.*)/gim, ($0, $1, $2, $3) => {
      return `await dk.setMaxColumns ({count: ${$3}})\n`;
    }];

    keywords[i++] = [/^\s*(set translator)(\s*)(.*)/gim, ($0, $1, $2, $3) => {
      return `await dk.setTranslatorOn ({on: "${$3.toLowerCase()}"})\n`;
    }];

    keywords[i++] = [/^\s*(set theme)(\s*)(.*)/gim, ($0, $1, $2, $3) => {
      return `await dk.setTheme ({theme: "${$3.toLowerCase()}"})\n`;
    }];

    keywords[i++] = [/^\s*(set whole word)(\s*)(.*)/gim, ($0, $1, $2, $3) => {
      return `await dk.setWholeWord ({on: "${$3.toLowerCase()}"})\n`;
    }];

    keywords[i++] = [/^\s*(\w+)\s*\=\s*post\s*(.*),\s*(.*)/gim, ($0, $1, $2, $3) => {
      return `${$1} = await sys.postByHttp ({url:${$2}, data:${$3}, headers})`;
    }];

    keywords[i++] = [/^\s*(\w+)\s*\=\s*put\s*(.*),\s*(.*)/gim, ($0, $1, $2, $3) => {
      return `${$1} = await sys.putByHttp ({url:${$2}, data:${$3}, headers})`;
    }];

    keywords[i++] = [/^\s*(\w+)\s*\=\s*download\s*(.*),\s*(.*)/gim, ($0, $1, $2, $3) => {
      return `${$1} = await sys.download ({handle:page, selector: ${$2}, folder:${$3}})`;
    }];

    keywords[i++] = [/^\s*(\w+)\s*\=\s*CREATE FOLDER\s*(.*)/gim, ($0, $1, $2) => {
      return `${$1} = await sys.createFolder ({name:${$2}})`;
    }];

    keywords[i++] = [/^\sSHARE FOLDER\s*(.*)/gim, ($0, $1) => {
      return `await sys.shareFolder ({name: ${$1}})`;
    }];

    keywords[i++] = [/^\s*(create a bot farm using)(\s)(.*)/gim, ($0, $1, $2, $3) => {
      return `await sys.createABotFarmUsing ({${$3}})`;
    }];

    keywords[i++] = [/^\s*(transfer to)(\s)(.*)/gim, ($0, $1, $2, $3) => {
      return `await dk.transferTo ({to:${$3}})\n`;
    }];

    keywords[i++] = [/^\s*(\btransfer\b)(?=(?:[^"]|"[^"]*")*$)/gim, () => {
      return `await dk.transferTo ({})\n`;
    }];

    keywords[i++] = [/^\s*(exit)/gim, () => {
      return ``;
    }];

    keywords[i++] = [/^\s*(show menu)/gim, () => {
      return `await dk.showMenu ({})\n`;
    }];

    keywords[i++] = [/^\s*(talk to)(\s)(.*)/gim, ($0, $1, $2, $3) => {
      const params = this.getParams($3, ['mobile', 'message']);
      return `await sys.talkTo(${params})\n`;
    }];

    keywords[i++] = [/^\s*(talk)(\s)(.*)/gim, ($0, $1, $2, $3) => {
      if ($3.substr(0, 1) !== "\"") {
        $3 = `"${$3}"`;
      }
      return `await dk.talk ({text: ${$3}})\n`;
    }];

    keywords[i++] = [/^\s*(send sms to)(\s*)(.*)/gim, ($0, $1, $2, $3) => {
      const params = this.getParams($3, ['mobile', 'message']);
      return `await sys.sendSmsTo(${params})\n`;
    }];

    keywords[i++] = [/^\s*(send email)(\s*)(.*)/gim, ($0, $1, $2, $3) => {
      const params = this.getParams($3, ['to', 'subject', 'body']);
      return `await dk.sendEmail(${params})\n`;
    }];

    keywords[i++] = [/^\s*(send mail)(\s*)(.*)/gim, ($0, $1, $2, $3) => {
      const params = this.getParams($3, ['to', 'subject', 'body']);
      return `await dk.sendEmail(${params})\n`;
    }];

    keywords[i++] = [/^\s*(send file to)(\s*)(.*)/gim, ($0, $1, $2, $3) => {
      const params = this.getParams($3, ['mobile', 'filename', 'caption']);
      return `await dk.sendFileTo(${params})\n`;
    }];

    keywords[i++] = [/^\s*(hover)(\s*)(.*)/gim, ($0, $1, $2, $3) => {
      const params = this.getParams($3, ['handle', 'selector']);
      return `await wa.hover (${params})\n`;
    }];

    keywords[i++] = [/^\s*(click link text)(\s*)(.*)/gim, ($0, $1, $2, $3) => {
      const params = this.getParams('page,' + $3, ['handle', 'text', 'index']);
      return `await wa.linkByText (${params})\n`;
    }];

    keywords[i++] = [/^\s*(click)(\s*)(.*)/gim, ($0, $1, $2, $3) => {
      // TODO: page is not string.
      const params = this.getParams('page,' + $3, ['handle', 'frameOrSelector', 'selector']);
      return `await wa.click (${params})\n`;
    }];

    keywords[i++] = [/^\s*(send file)(\s*)(.*)/gim, ($0, $1, $2, $3) => {
      const params = this.getParams($3, ['filename', 'caption']);
      return `await dk.sendFile(${params})\n`;
    }];

    keywords[i++] = [/^\s*(copy)(\s*)(.*)/gim, ($0, $1, $2, $3) => {
      const params = this.getParams($3, ['src', 'dst']);
      return `await sys.copyFile (${params})\n`;
    }];

    keywords[i++] = [/^\s*(convert)(\s*)(.*)/gim, ($0, $1, $2, $3) => {
      const params = this.getParams($3, ['src', 'dst']);
      return `await sys.convert (${params})\n`;
    }];

    keywords[i++] = [/^\s*(\w+)\s*\=\s*(.*)\s*as chart/gim, ($0, $1, $2) => {
      return `await dk.chart ({type:'bar', data: ${2}, legends:null, transpose: false})\n`;
    }];

    keywords[i++] = [/^\s*(chart)(\s)(.*)/gim, ($0, $1, $2, $3) => {
      const params = this.getParams($3, ['type', 'data', 'legends', 'transpose']);
      return `await dk.chart (${params})\n`;
    }];

    keywords[i++] = [/^\sMERGE\s(.*)\sWITH\s(.*)BY\s(.*)/gim, ($0, $1, $2, $3) => {
      return `await sys.merge({file: ${$1}, data: ${$2}, key1: ${$3}})\n`;
    }];

    keywords[i++] = [/^\sPRESS\s(.*)/gim, ($0, $1, $2) => {
      return `await wa.pressKey({handle: page, char: ${$1})\n`;
    }];

    keywords[i++] = [/^\sSCREENSHOT\s(.*)/gim, ($0, $1, $2) => {
      return `await wa.screenshot({handle: page, selector: ${$1}})\n`;
    }];

    keywords[i++] = [/^\sTWEET\s(.*)/gim, ($0, $1, $2) => {
      return `await sys.tweet({text: ${$1})\n`;
    }];

    keywords[i++] = [/^\s*(\w+)\s*\=\s*(.*)\s*as image/gim, ($0, $1, $2) => {
      return `${$1} = await sys.asImage({data: ${$2}})\n`;
    }];

    keywords[i++] = [/^\s*(\w+)\s*\=\s*(.*)\s*as pdf/gim, ($0, $1, $2) => {
      return `${$1} = await sys.asPdf({data: ${$2})\n`;
    }];

    keywords[i++] = [/^\s*(\w+)\s*\=\s*FILL\s(.*)\sWITH\s(.*)/gim, ($0, $1, $2, $3) => {
      return `${$1} = await sys.fill({templateName: ${$2}, data: ${$3}})\n`;
    }];

    keywords[i++] = [/^\ssave\s(.*)\sas\s(.*)/gim, ($0, $1, $2, $3) => {
      return `await sys.saveFile({file: ${$2}, data: ${$1})\n`;
    }];
    keywords[i++] = [/^\s*(save)(\s)(.*)/gim, ($0, $1, $2, $3) => {
      return `await sys.save({args: [${$3}]})\n`;
    }];

    keywords[i++] = [/^\sset\s(.*)/gim, ($0, $1, $2) => {
      const params = this.getParams($1, ['file', 'address', 'value']);
      return `await sys.set (${params})`;
    }];
    return keywords;
  }


  /**
   * Executes the converted JavaScript from BASIC code inside execution context.
   */
  public static async callVM(text: string, min: GBMinInstance, step, GBDialogdeployer: GBDeployer) {

    // Creates a class DialogKeywords which is the *this* pointer
    // in BASIC.

    const user = step   ? await min.userProfile.get(step.context, {}) : null;

    const sandbox = { user: user.systemUser };

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

    if (GBConfigService.get('VM3') === 'true') {
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
      const runnerPath = urlJoin(process.cwd(), 'dist', 'packages', 'basic.gblib', 'services', 'vm2-process', 'vm2ProcessRunner.js');

      try {
        const { run } = createVm2Pool({
          min: 0,
          max: 0,
          debuggerPort: 9222,
          botId: botId,
          cpu: 100,
          memory: 50000,
          time: 60 * 60 * 24 * 14,
          cwd: gbdialogPath,
          script: runnerPath
        });

        const port = run.port;
        const result = await run(code, { filename: scriptPath, sandbox: sandbox });
        
        return result;
      } catch (error) {
        throw new Error(`BASIC RUNTIME ERR: ${error.message ? error.message : error}\n Stack:${error.stack}`);
      }
    }
  }
}
