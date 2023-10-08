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

import { GBMinInstance, GBService, IGBCoreService, GBDialogStep } from 'botlib';
import * as Fs from 'fs';
import { GBServer } from '../../../src/app.js';
import { GBDeployer } from '../../core.gbapp/services/GBDeployer.js';
import { CollectionUtil } from 'pragmatismo-io-framework';
import { ScheduleServices } from './ScheduleServices.js';
import { GBConfigService } from '../../core.gbapp/services/GBConfigService.js';
import urlJoin from 'url-join';
import { NodeVM, VMScript } from 'vm2';
import { createVm2Pool } from './vm2-process/index.js';
import textract from 'textract';
import walkPromise from 'walk-promise';
import child_process from 'child_process';
import Path from 'path';
import { GBAdminService } from '../../admin.gbapp/services/GBAdminService.js';
import { DialogKeywords } from './DialogKeywords.js';
import { KeywordsExpressions } from './KeywordsExpressions.js';
import { GBLogEx } from '../../core.gbapp/services/GBLogEx.js';
import { GuaribasUser } from '../../security.gbapp/models/index.js';
import { SystemKeywords } from './SystemKeywords.js';
import lineReplace from 'line-replace';
import { Sequelize, DataTypes } from '@sequelize/core';
import { table } from 'console';

/**
 * @fileoverview  Decision was to priorize security(isolation) and debugging,
 * over a beautiful BASIC transpiler (to be done).
 */

/**
 * Basic services for BASIC manipulation.
 */
export class GBVMService extends GBService {
  private static DEBUGGER_PORT = 9222;
  public static API_PORT = 1111;

  public async loadDialogPackage(folder: string, min: GBMinInstance, core: IGBCoreService, deployer: GBDeployer) {
    const files = await walkPromise(folder);

    await CollectionUtil.asyncForEach(files, async file => {
      if (!file) {
        return;
      }

      let filename: string = file.name;

      if (filename.endsWith('.docx')) {
        filename = await this.loadDialog(filename, folder, min);
      }
    });
  }

  public async loadDialog(filename: string, folder: string, min: GBMinInstance) {
    const wordFile = filename;
    const vbsFile = filename.substr(0, filename.indexOf('docx')) + 'vbs';
    const fullVbsFile = urlJoin(folder, vbsFile);
    const docxStat = Fs.statSync(urlJoin(folder, wordFile));
    const interval = 3000; // If compiled is older 30 seconds, then recompile.
    let writeVBS = true;

    if (Fs.existsSync(fullVbsFile)) {
      const vbsStat = Fs.statSync(fullVbsFile);
      if (docxStat['mtimeMs'] < vbsStat['mtimeMs'] + interval) {
        writeVBS = false;
      }
    }
    filename = vbsFile;
    let mainName = GBVMService.getMethodNameFromVBSFilename(filename);
    min.scriptMap[filename] = mainName;

    if (writeVBS) {
      let text = await this.getTextFromWord(folder, wordFile);

      // Pre process SET SCHEDULE calls.

      const schedule = GBVMService.getSetScheduleKeywordArgs(text);
      const s = new ScheduleServices();
      if (schedule) {
        await s.createOrUpdateSchedule(min, schedule, mainName);
      } else {
        await s.deleteScheduleIfAny(min, mainName);
      }
      text = text.replace(/^\s*SET SCHEDULE (.*)/gim, '');

      // Write VBS file without pragma keywords.

      Fs.writeFileSync(urlJoin(folder, vbsFile), text);
    }

    // Process node_modules install.

    const node_modules = urlJoin(process.env.PWD, folder, 'node_modules');
    if (!Fs.existsSync(node_modules)) {
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
                "@push-rpc/core": "1.1.5",
                "@push-rpc/http": "1.1.5",
                "vm2": "3.9.11"
              }
            }`;
      Fs.writeFileSync(urlJoin(folder, 'package.json'), packageJson);

      GBLogEx.info(min, `BASIC: Installing .gbdialog node_modules for ${min.botId}...`);
      const npmPath = urlJoin(process.env.PWD, 'node_modules', '.bin', 'npm');
      child_process.execSync(`${npmPath} install`, { cwd: folder });
    }

    // Hot swap for .vbs files.

    const fullFilename = urlJoin(folder, filename);
    if (process.env.DEV_HOTSWAP) {
      Fs.watchFile(fullFilename, async () => {
        await this.translateBASIC(mainName, fullFilename, min);
        const parsedCode: string = Fs.readFileSync(jsfile, 'utf8');
        min.sandBoxMap[mainName.toLowerCase().trim()] = parsedCode;
      });
    }

    const compiledAt = Fs.statSync(fullFilename);
    const jsfile = urlJoin(folder, `${filename}.js`);

    if (Fs.existsSync(jsfile)) {
      const jsStat = Fs.statSync(jsfile);
      const interval = 30000; // If compiled is older 30 seconds, then recompile.
      if (compiledAt.isFile() && compiledAt['mtimeMs'] > jsStat['mtimeMs'] + interval) {
        await this.translateBASIC(mainName, fullFilename, min);
      }
    } else {
      await this.translateBASIC(mainName, fullFilename, min);
    }

    // Syncronizes Database Objects with the ones returned from "Word".

    const tablesFile = urlJoin(folder, `${filename}.tables.json`);
    if (Fs.existsSync(tablesFile)) {
      const minBoot = GBServer.globals.minBoot;
      GBLogEx.info(min, `BASIC: Sync TABLE keywords storage for ${min.botId}...`);

      const t = JSON.parse(Fs.readFileSync(tablesFile, 'utf8'));

      const getTypeBasedOnCondition = (t) => {
        switch (t) {
          case 'string':
            return { key: 'STRING' };
          case 'guid':
            return { key: 'UUID' };
          case 'key':
            return { key: 'STRING' }; // Assuming key is a string data type
          case 'integer':
            return { key: 'INTEGER' };
          case 'double':
            return { key: 'FLOAT' };
          case 'date':
            return { key: 'DATE' };
          case 'boolean':
            return { key: 'BOOLEAN' };
          default:
            return { key: 'TABLE' , name: t};
        }
      };

      const associations = [];

      Object.keys(t.fields).forEach(key => {
        let obj = t.fields[key];
        obj.type = getTypeBasedOnCondition(obj.type);
        if (obj.type.key === "TABLE"){
          associations.push({from: t.name,to: obj.type.name});
        }
        if (obj.name.toLowerCase() === 'id')
        {
          obj['primaryKey'] = true;        
        }
  
      });

      associations.forEach(e=>{
        const from = minBoot.core.sequelize.models[e.from];
        const to = minBoot.core.sequelize.models[e.to];

        from.hasMany(to);
        to.belongsTo(from);

      });

      minBoot.core.sequelize.define(t.name, t.fields);

      await minBoot.core.sequelize.sync({
        alter: true,
        force: false // Keep it false due to data loss danger.
      });
    }

    const parsedCode: string = Fs.readFileSync(jsfile, 'utf8');
    min.sandBoxMap[mainName.toLowerCase().trim()] = parsedCode;
    return filename;

  }

  public async translateBASIC(mainName, filename: any, min: GBMinInstance) {

    // Converts General Bots BASIC into regular VBS

    let basicCode: string = Fs.readFileSync(filename, 'utf8');

    // Process INCLUDE keyword to include another
    // dialog inside the dialog.

    let include = null;
    do {
      include = /^include\b(.*)$/gim.exec(basicCode);

      if (include) {
        let includeName = include[1].trim();
        includeName = Path.join(Path.dirname(filename), includeName);
        includeName = includeName.substr(0, includeName.lastIndexOf('.')) + '.vbs';

        // To use include, two /publish will be necessary (for now)
        // because of alphabet order may raise not found errors.

        let includeCode: string = Fs.readFileSync(includeName, 'utf8');
        basicCode = basicCode.replace(/^include\b.*$/gim, includeCode);
      }
    } while (include);

    let { code, map, metadata, tasks } = await this.convert(filename, mainName, basicCode);

    // Generates function JSON metadata to be used later.

    const jsonFile = `${filename}.json`;
    Fs.writeFileSync(jsonFile, JSON.stringify(metadata));

    const mapFile = `${filename}.map`;

    Fs.writeFileSync(mapFile, JSON.stringify(map));

    // Execute off-line code tasks

    await this.executeTasks(min, tasks);

    // Run JS into the GB context.

    const jsfile: string = `${filename}.js`;

    code = `
    return (async () => {

      // Imports npm packages for this .gbdialog conversational application.

      require('isomorphic-fetch');
      const createRpcClient = require("@push-rpc/core").createRpcClient;
      const createHttpClient = require("@push-rpc/http").createHttpClient;
      
      // Unmarshalls Local variables from server VM.

      const pid = this.pid;
      let id = this.id;
      let username = this.username;
      let mobile = this.mobile;
      let from = this.from;
      const channel = this.channel;
      const ENTER = this.ENTER;
      const headers = this.headers;
      let data = this.data;
      let list = this.list;
      let httpUsername = this.httpUsername;
      let httpPs = this.httpPs;
      let today = this.today;
      let now = this.now;
      let page = null;
      const files = [];
      let col = 1;
      let index = 1;

      // Makes objects in BASIC insensitive.

      const caseInsensitive = (listOrRow) => {
        
        if (!listOrRow) {
          
          return listOrRow;
        };

        const lowercase = (oldKey) => typeof oldKey === 'string' ? oldKey.toLowerCase() : oldKey;

        const createCaseInsensitiveProxy = (obj) => {
            const propertiesMap = new Map(Object.keys(obj).map(propKey => [lowercase(propKey), obj[propKey]]));
            const caseInsensitiveGetHandler = {
                get: (target, property) => propertiesMap.get(lowercase(property))
            };
            return new Proxy(obj, caseInsensitiveGetHandler);
        };

        if (listOrRow.length) {
            return listOrRow.map(row => createCaseInsensitiveProxy(row));
        } else {
            return createCaseInsensitiveProxy(listOrRow);
        }
      };

      // Transfers NLP auto variables into global object.

      for(i in this.variables) { 
          global[i] = this.variables[i];
      }   

      // Defines local utility BASIC functions.

      const ubound = (gbarray) => {return gbarray ? gbarray.length - 1: 0};
      const isarray = (gbarray) => {return Array.isArray(gbarray) };
  
      // Proxies remote functions as BASIC functions.
      
      const weekday = (v) => { return (async () => { return await dk.getWeekFromDate({v}) })(); };
      const hour = (v) => { return (async () => { return await dk.getHourFromDate({v}) })(); };
      const base64 =  (v) => { return (async () => { return await dk.getCoded({v}) })(); };
      const tolist =  (v) => { return (async () => { return await dk.getToLst({v}) })(); };
      const uuid =  () => { 
          var dt = new Date().getTime();
          var uuid = 'xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx'.replace(/[xy]/g, (c) => {
              var r = (dt + Math.random()*16)%16 | 0;
              dt = Math.floor(dt/16);
              return (c=='x' ? r :(r&0x3|0x8)).toString(16);
          });
          return uuid;
      };

      // Setups interprocess communication from .gbdialog run-time to the BotServer API.

      const optsRPC = {callTimeout: this.callTimeout};
      let url;

      url = 'http://localhost:${GBVMService.API_PORT}/api/v3/${min.botId}/dk';
      const dk = (await createRpcClient(0, () => createHttpClient(url), optsRPC)).remote;
      url = 'http://localhost:${GBVMService.API_PORT}/api/v3/${min.botId}/sys';
      const sys =  (await createRpcClient(0, () => createHttpClient(url), optsRPC)).remote;
      url = 'http://localhost:${GBVMService.API_PORT}/api/v3/${min.botId}/wa';
      const wa = (await createRpcClient(0, () => createHttpClient(url), optsRPC)).remote;
      url = 'http://localhost:${GBVMService.API_PORT}/api/v3/${min.botId}/img';
      const img =  (await createRpcClient(0, () => createHttpClient(url), optsRPC)).remote;
  
      ${code}

      // Closes handles if any.

      await wa.closeHandles({pid: pid});

    })(); 
`;
    Fs.writeFileSync(jsfile, code);
    GBLogEx.info(min, `[GBVMService] Finished loading of ${filename}, JavaScript from Word: \n ${code}`);

  }

  private async executeTasks(min, tasks) {
    for (let i = 0; i < tasks.length; i++) {
      const task = tasks[i];

      if (task.kind === 'writeTableDefinition') {

        // Creates an empty object that will receive Sequelize fields.

        let obj = { name: task.name };
        obj['fields'] = task.fields;
        const path = DialogKeywords.getGBAIPath(min.botId, `gbdialog`);
        const tablesFile = `${task.file}.tables.json`;

        Fs.writeFileSync(tablesFile, JSON.stringify(obj));

      }

    }
  }

  public static getMethodNameFromVBSFilename(filename: string) {
    let mainName = filename.replace(/\s*|\-/gim, '').split('.')[0];
    return mainName.toLowerCase();
  }

  public static getSetScheduleKeywordArgs(code: string) {
    if (!code) return null;
    const keyword = /^\s*SET SCHEDULE (.*)/gim;
    const result = keyword.exec(code);
    return result ? result[1] : null;
  }

  private async getTextFromWord(folder: string, filename: string) {
    return new Promise<string>(async (resolve, reject) => {
      const path = urlJoin(folder, filename);
      textract.fromFileWithPath(path, { preserveLineBreaks: true }, (error, text) => {
        if (error) {
          if (error.message.startsWith('File not correctly recognized as zip file')) {
            text = Fs.readFileSync(path, 'utf8');
          } else {
            reject(error);
          }
        }

        if (text) {
          text = GBVMService.normalizeQuotes(text);
        }
        resolve(text);
      });
    });
  }

  public static normalizeQuotes(text: any) {
    text = text.replace(/\¨/gm, '"');
    text = text.replace(/\“/gm, '"');
    text = text.replace(/\”/gm, '"');
    text = text.replace(/\‘/gm, "'");
    text = text.replace(/\’/gm, "'");

    return text;
  }

  public static getMetadata(mainName: string, propertiesText, description) {
    const properties = [];

    if (propertiesText) {
      const getType = asClause => {
        if (asClause.indexOf('AS STRING')) {
          return 'string';
        } else {
          return 'enum';
        }
      };

      for (let i = 0; i < propertiesText.length; i++) {
        const propertiesExp = propertiesText[i];
        const t = getType(propertiesExp[2]);
        let element = {};
        element['type'] = t;

        if (t === 'enum') {
          element['enum'] = propertiesExp[2];
        } else if (t === 'string') {
          element['description'] = propertiesExp[2];
        }

        properties.push(element);
      }
    }

    let json = {
      name: `${mainName}`,
      description: description ? description[1] : '',
      parameters: {
        type: 'object',
        properties: properties ? properties : []
      }
    };

    return json;
  }

  public async parseField(line) {

    let required = line.indexOf('*') !== -1;
    line = line.replace('*', '');

    const fieldRegExp = /^\s*(\w+)\s*(\w+)(?:\((.*)\))?/gim;

    let reg = fieldRegExp.exec(line);
    const t = reg[2];
    const name = reg[1];

    let definition = { allowNull: !required };
    definition['type'] = t;

    if (reg[3]) {
      definition['size'] = Number.parseInt(reg[3] === 'max' ? '4000' : reg[3]);
    }

    return { name, definition };
  }

  /**
   * Converts General Bots BASIC
   *
   *
   * @param code General Bots BASIC
   */
  public async convert(filename: string, mainName: string, code: string) {

    // Start and End of VB2TS tags of processing.

    code = process.env.ENABLE_AUTH ? `hear GBLogExin as login\n${code}` : code;
    var lines = code.split('\n');
    const keywords = KeywordsExpressions.getKeywords();
    let current = 41;
    const map = {};
    let properties = [];
    let description;
    let table = null; // Used for TABLE keyword.
    const tasks = [];
    let fields = {};

    for (let i = 1; i <= lines.length; i++) {

      let line = lines[i - 1];

      // Remove lines before statments.

      line = line.replace(/^\s*\d+\s*/gi, '');

      for (let j = 0; j < keywords.length; j++) {
        line = line.replace(keywords[j][0], keywords[j][1]);
      }

      // Pre-process "off-line" static KEYWORDS.

      let emmit = true;
      const params = /^\s*PARAM\s*(.*)\s*AS\s*(.*)/gim;
      const param = params.exec(line);
      if (param) {
        properties.push(param);
        emmit = false;
      }

      const descriptionKeyword = /^\s*DESCRIPTION\s*\"(.*)\"/gim;
      let descriptionReg = descriptionKeyword.exec(line);
      if (descriptionReg) {
        description = descriptionReg[1];
        emmit = false;
      }

      const endTableKeyword = /^\s*END TABLE\s*/gim;
      let endTableReg = endTableKeyword.exec(line);
      if (endTableReg && table) {

        tasks.push({
          kind: 'writeTableDefinition', file: filename, name: table, fields: fields
        });

        fields = [];
        table = null;
        emmit = false;
      }

      // Inside BEGIN/END table pair containing FIELDS.

      if (table && line.trim() !== '') {
        const field = await this.parseField(line);
        fields[field.name] = field.definition;
        emmit = false;
      }

      const tableKeyword = /^\s*TABLE\s*(.*)/gim;
      let tableReg = tableKeyword.exec(line);
      if (tableReg && !table) {
        table = tableReg[1];
        emmit = false;
      }

      //  Add additional lines returned from replacement.

      let add = emmit ? line.split(/\r\n|\r|\n/).length : 0;
      current = current + (add ? add : 0);
      map[i] = current;
      lines[i - 1] = emmit ? line : '';
    }

    code = `${lines.join('\n')}\n`;

    let metadata = GBVMService.getMetadata(mainName, properties, description);

    return { code, map, metadata, tasks };
  }

  /**
   * Executes the converted JavaScript from BASIC code inside execution context.
   */
  public static async callVM(
    text: string,
    min: GBMinInstance,
    step,
    user: GuaribasUser,
    deployer: GBDeployer,
    debug: boolean = false,
    params = []
  ) {
    // Creates a class DialogKeywords which is the *this* pointer
    // in BASIC.

    const sandbox = {};
    const contentLocale = min.core.getParam<string>(
      min.instance,
      'Default Content Language',
      GBConfigService.get('DEFAULT_CONTENT_LANGUAGE')
    );

    // These variables will be automatically be available as normal BASIC variables.

    let variables = [];

    variables['aadToken'] = await (min.adminService as any)['acquireElevatedToken'](min.instance.instanceId, false);

    // Adds all .gbot params as variables.

    const gbotConfig = JSON.parse(min.instance.params);
    let keys = Object.keys(gbotConfig);
    for (let j = 0; j < keys.length; j++) {
      const name = keys[j].replace(/\s/gi, '');
      variables[name] = gbotConfig[keys[j]];
    }

    // Auto-NLP generates BASIC variables related to entities.

    if (step ? step.context.activity.originalText : null && min['nerEngine']) {
      const result = await min['nerEngine'].process(step.context.activity.originalText);

      for (let i = 0; i < result.entities.length; i++) {
        const v = result.entities[i];
        const variableName = `${v.entity}`;
        variables[variableName] = v.option ? v.option : v.sourceText;
      }
    }

    // Adds params as variables to be added later as global objects..

    keys = Object.keys(params);
    for (let j = 0; j < keys.length; j++) {
      variables[keys[j]] = params[keys[j]];
    }

    const botId = min.botId;
    const path = DialogKeywords.getGBAIPath(min.botId, `gbdialog`);
    const gbdialogPath = urlJoin(process.cwd(), 'work', path);
    const scriptPath = urlJoin(gbdialogPath, `${text}.js`);

    let code = min.sandBoxMap[text];
    const channel = step ? step.context.activity.channelId : 'web';
    const pid = GBVMService.createProcessInfo(user, min, channel, text);
    const dk = new DialogKeywords();
    const sys = new SystemKeywords();
    await dk.setFilter({ pid: pid, value: null });

    sandbox['variables'] = variables;
    sandbox['id'] = sys.getRandomId();
    sandbox['username'] = await dk.userName({ pid });
    sandbox['mobile'] = await dk.userMobile({ pid });
    sandbox['from'] = await dk.userMobile({ pid });
    sandbox['ENTER'] = String.fromCharCode(13);
    sandbox['headers'] = {};
    sandbox['data'] = {};
    sandbox['list'] = [];
    sandbox['httpUsername'] = '';
    sandbox['httpPs'] = '';
    sandbox['pid'] = pid;
    sandbox['contentLocale'] = contentLocale;
    sandbox['callTimeout'] = 60 * 60 * 24 * 1000;
    sandbox['channel'] = channel;
    sandbox['today'] = await dk.getToday({ pid });
    sandbox['now'] = await dk.getNow({ pid });
    let result;

    try {
      if (GBConfigService.get('GBVM') === 'false') {
        const vm1 = new NodeVM({
          allowAsync: true,
          sandbox: sandbox,
          console: 'inherit',
          wrapper: 'commonjs',
          require: {
            builtin: ['stream', 'http', 'https', 'url', 'zlib', 'net', 'tls', 'crypto'],
            root: ['./'],
            external: true,
            context: 'sandbox'
          }
        });
        const s = new VMScript(code, { filename: scriptPath });
        result = vm1.run(s);
      } else {
        const runnerPath = urlJoin(
          process.cwd(),
          'dist',
          'packages',
          'basic.gblib',
          'services',
          'vm2-process',
          'vm2ProcessRunner.js'
        );

        const { run } = createVm2Pool({
          min: 0,
          max: 0,
          debug: debug,
          debuggerport: GBVMService.DEBUGGER_PORT,
          botId: botId,
          cpu: 100,
          memory: 50000,
          time: 60 * 60 * 24 * 14,
          cwd: gbdialogPath,
          script: runnerPath
        });

        result = await run(code, { filename: scriptPath, sandbox: sandbox });
      }
    } catch (error) {
      throw new Error(`BASIC RUNTIME ERR: ${error.message ? error.message : error}\n Stack:${error.stack}`);
    }

    return result;
  }

  public static createProcessInfo(user: GuaribasUser, min: GBMinInstance, channel: any, executable: string) {
    const pid = GBAdminService.getNumberIdentifier();
    GBServer.globals.processes[pid] = {
      pid: pid,
      userId: user ? user.userId : 0,
      instanceId: min.instance.instanceId,
      channel: channel,
      roles: 'everyone',
      executable: executable
    };
    return pid;
  }
}
