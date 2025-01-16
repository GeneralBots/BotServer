/*****************************************************************************\
|  █████  █████ ██    █ █████ █████   ████  ██      ████   █████ █████  ███ ® |
| ██      █     ███   █ █     ██  ██ ██  ██ ██      ██  █ ██   ██  █   █      |
| ██  ███ ████  █ ██  █ ████  █████  ██████ ██      ████   █   █   █    ██    |
| ██   ██ █     █  ██ █ █     ██  ██ ██  ██ ██      ██  █ ██   ██  █      █   |
|  █████  █████ █   ███ █████ ██  ██ ██  ██ █████   ████   █████   █   ███    |
|                                                                             |
| General Bots Copyright (c) pragmatismo.cloud. All rights reserved.          |
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
| "General Bots" is a registered trademark of pragmatismo.cloud.              |
| The licensing of the program under the AGPLv3 does not imply a              |
| trademark license. Therefore any rights, title and interest in              |
| our trademarks remain entirely with us.                                     |
|                                                                             |
\*****************************************************************************/

'use strict';

import { GBMinInstance, GBService, IGBCoreService, GBLog } from 'botlib';
import fs from 'fs/promises';
import * as ji from 'just-indent';
import { GBServer } from '../../../src/app.js';
import { GBDeployer } from '../../core.gbapp/services/GBDeployer.js';
import { CollectionUtil } from 'pragmatismo-io-framework';
import { ScheduleServices } from './ScheduleServices.js';
import { GBConfigService } from '../../core.gbapp/services/GBConfigService.js';
import urlJoin from 'url-join';
import { PostgresDialect } from '@sequelize/postgres';
import { NodeVM, VMScript } from 'vm2';
import { createVm2Pool } from './vm2-process/index.js';
import { watch } from 'fs';
import textract from 'textract';
import walkPromise from 'walk-promise';
import child_process from 'child_process';
import path from 'path';
import { GBAdminService } from '../../admin.gbapp/services/GBAdminService.js';
import { DialogKeywords } from './DialogKeywords.js';
import { KeywordsExpressions } from './KeywordsExpressions.js';
import { GBLogEx } from '../../core.gbapp/services/GBLogEx.js';
import { GuaribasUser } from '../../security.gbapp/models/index.js';
import { SystemKeywords } from './SystemKeywords.js';
import { Sequelize, QueryTypes } from '@sequelize/core';
import { z } from 'zod';
import { zodToJsonSchema } from 'zod-to-json-schema';
import { GBUtil } from '../../../src/util.js';

/**
 * @fileoverview  Decision was to priorize security(isolation) and debugging,
 * over a beautiful BASIC transpiler (to be done).
 */

/**
 * Basic services for BASIC manipulation.
 */
export class GBVMService extends GBService {
  public static API_PORT = 1111;

  public async loadDialogPackage(folder: string, min: GBMinInstance, core: IGBCoreService, deployer: GBDeployer) {
    const ignore = path.join('work', GBUtil.getGBAIPath(min.botId, 'gbdialog'), 'node_modules');
    const files = await walkPromise(folder, { ignore: [ignore] });

    await CollectionUtil.asyncForEach(files, async file => {
      if (!file) {
        return;
      }

      let filename: string = file.name;

      filename = await this.loadDialog(filename, folder, min);
    });
  }

  public static compare(obj1, obj2) {
    //check for obj2 overlapping props
    if (!Object.keys(obj2).every(key => obj1.hasOwnProperty(key))) {
      return false;
    }

    //check every key for being same
    return Object.keys(obj1).every(function (key) {
      //if object
      if (typeof obj1[key] == 'object' && typeof obj2[key] == 'object') {
        //recursively check
        return GBVMService.compare(obj1[key], obj2[key]);
      } else {
        //do the normal compare
        return obj1[key] === obj2[key];
      }
    });
  }

  public async loadDialog(filename: string, folder: string, min: GBMinInstance) {
    const isWord = filename.endsWith('.docx');
    if (
      !(
        isWord ||
        filename.endsWith('.vbs') ||
        filename.endsWith('.vb') ||
        filename.endsWith('.vba') ||
        filename.endsWith('.bas') ||
        filename.endsWith('.basic')
      )
    ) {
      return;
    }

    const wordFile = filename;
    const vbsFile = isWord ? filename.substr(0, filename.indexOf('docx')) + 'vbs' : filename;
    const fullVbsFile = urlJoin(folder, vbsFile);
    const docxStat = await fs.stat(urlJoin(folder, wordFile));
    const interval = 3000; // If compiled is older 30 seconds, then recompile.
    let writeVBS = true;

    // TODO: #412.
    // const subscription = {
    //   changeType: 'created,updated',
    //   notificationUrl: 'https://webhook.azurewebsites.net/notificationClient',
    //   lifecycleNotificationUrl: 'https://webhook.azurewebsites.net/api/lifecycleNotifications',
    //   resource: '/me/mailfolders(\'inbox\')/messages',
    //   expirationDateTime: '2016-03-20T11:00:00.0000000Z',
    //   clientState: 'SecretClientState'
    // };

    // let { baseUrl, client } = await GBDeployer.internalGetDriveClient(min);

    // await client.api('/subscriptions')
    //   .post(subscription);

    if (await GBUtil.exists(fullVbsFile)) {
      const vbsStat = await fs.stat(fullVbsFile);
      if (docxStat['mtimeMs'] < vbsStat['mtimeMs'] + interval) {
        writeVBS = false;
      }
    }
    filename = vbsFile;
    let mainName = GBVMService.getMethodNameFromVBSFilename(filename);
    min.scriptMap[filename] = mainName;

    if (writeVBS && GBConfigService.get('STORAGE_NAME')) {
      let text = await this.getTextFromWord(folder, wordFile);

      // Write VBS file without pragma keywords.

      await fs.writeFile(urlJoin(folder, vbsFile), text);
    }

    // Process node_modules install.

    await this.processNodeModules(folder, min);

    // Hot swap for .vbs files.

    const fullFilename = urlJoin(folder, filename);
    if (process.env.DEV_HOTSWAP) {
      watch(fullFilename, async () => {
        await this.translateBASIC(mainName, fullFilename, min);
        const parsedCode: string = await fs.readFile(jsfile, 'utf8');
        min.sandBoxMap[mainName.toLowerCase().trim()] = parsedCode;
      });
    }

    const compiledAt = await fs.stat(fullFilename);
    const jsfile = urlJoin(folder, `${filename}.js`);

    if (await GBUtil.exists(jsfile)) {
      const jsStat = await fs.stat(jsfile);
      const interval = 1000; // If compiled is older 1 seconds, then recompile.
      if (compiledAt.isFile() && compiledAt['mtimeMs'] > jsStat['mtimeMs'] + interval) {
        await this.translateBASIC(mainName, fullFilename, min);
      }
    } else {
      await this.translateBASIC(mainName, fullFilename, min);
    }

    // Syncronizes Database Objects with the ones returned from "Word".

    await this.syncStorageFromTABLE(folder, filename, min, mainName);

    const parsedCode: string = await fs.readFile(jsfile, 'utf8');
    min.sandBoxMap[mainName.toLowerCase().trim()] = parsedCode;
    return filename;
  }
  private async processNodeModules(folder: string, min: GBMinInstance) {
    const node_modules = urlJoin(process.env.PWD, folder, 'node_modules');
    if (!(await GBUtil.exists(node_modules))) {
      const packageJson = `
            {
              "name": "${min.botId}.gbdialog",
              "version": "1.0.0",
              "description": "${min.botId} transpiled .gbdialog",
              "author": "${min.botId} owner.",
              "license": "ISC",
              "dependencies": {
                "yaml": "2.4.2",
                "encoding": "0.1.13",
                "isomorphic-fetch": "3.0.0",
                "punycode": "2.1.1",
                "@push-rpc/core": "1.8.2",
                "@push-rpc/http": "1.8.2",
                "vm2": "3.9.11",
                "async-retry": "1.3.3"
              }
            }`;
      await fs.writeFile(urlJoin(folder, 'package.json'), packageJson);

      GBLogEx.info(min, `Installing node_modules...`);
      const npmPath = urlJoin(process.env.PWD, 'node_modules', '.bin', 'npm');
      child_process.exec(`${npmPath} install`, { cwd: folder });
    }
  }

  public static async loadConnections(min) {
    // Loads storage custom connections.
    const packagePath = GBUtil.getGBAIPath(min.botId, null);
    const filePath = path.join('work', packagePath, 'connections.json');
    let connections = [];
    if (await GBUtil.exists(filePath)) {
      connections = JSON.parse(await fs.readFile(filePath, 'utf8'));
    }

    connections.forEach(async con => {
      const connectionName = con['name'];

      const dialect = con['storageDriver'];
      const host = con['storageServer'];
      const port = con['storagePort'];
      const storageName = con['storageName'];
      const username = con['storageUsername'];
      const password = con['storagePassword'];

      const logging: boolean | Function =
        GBConfigService.get('STORAGE_LOGGING') === 'true'
          ? (str: string): void => {
            GBLogEx.info(min, str);
          }
          : false;

      const encrypt: boolean = GBConfigService.get('STORAGE_ENCRYPT') === 'true';
      const acquire = parseInt(GBConfigService.get('STORAGE_ACQUIRE_TIMEOUT'));
      let sequelizeOptions;


      // Simple function to convert all object keys to lowercase
      const toLowerCase = (obj) => {
        if (!obj) return obj;
        if (typeof obj !== 'object') return obj;

        return Object.keys(obj).reduce((acc, key) => {
          acc[key.toLowerCase()] = obj[key];
          return acc;
        }, {});
      }

      if (dialect === 'postgres') {

        sequelizeOptions = {
          host: host,
          port: port,
          logging: logging as boolean,
          dialect: dialect,
          dialectOptions: {
            ssl: false,
            application_name: 'General Bots',
            connectTimeout: 10000,
            query_timeout: 10000,
            statement_timeout: 10000,
            idle_in_transaction_session_timeout: 10000,

          },
          pool: {
            max: 1,
            min: 0,
            idle: 10000,
            evict: 10000,
            acquire: acquire
          },
          define: {
            // Convert all table names to lowercase
            freezeTableName: true,
            hooks: {
              beforeDefine: (attributes, options) => {
    // Convert model name and table name to lowercase
    if (options.modelName) {
      options.modelName = options.modelName.toLowerCase();
  }
  if (options.tableName) {
      options.tableName = options.tableName.toLowerCase();
  } else {
      options.tableName = options.modelName.toLowerCase();
  }                for (const attr in attributes) {
                  const lowered = attr.toLowerCase();
                  if (attr !== lowered) {
                    attributes[lowered] = attributes[attr];
                    delete attributes[attr];
                  }
                }
              }
            }
          },

          // Convert query attributes to lowercase
          hooks: {
            beforeFind: (options) => {
              if (options.where) {
                options.where = toLowerCase(options.where);
              }
            }
          }
        };


      }
      else {

        sequelizeOptions = {
          define: {
            charset: 'utf8',
            collate: 'utf8_general_ci',
            freezeTableName: true,
            timestamps: false
          },
          host: host,
          port: port,
          logging: logging as boolean,
          dialect: dialect,
          quoteIdentifiers: false, // set case-insensitive
          dialectOptions: {
            options: {
              trustServerCertificate: true,
              encrypt: encrypt,
              requestTimeout: 120 * 1000
            }
          },
          pool: {
            max: 1,
            min: 0,
            idle: 10000,
            evict: 10000,
            acquire: acquire
          }
        };
      }

      if (!min[connectionName]) {
        GBLogEx.info(min, `Loading data connection ${connectionName} (${dialect})...`);
        min[connectionName] = new Sequelize(storageName, username, password, sequelizeOptions);
        min[connectionName]['gbconnection'] = con;
      }
    });
  }

  private async syncStorageFromTABLE(folder: string, filename: string, min: GBMinInstance, mainName: string) {
    const tablesFile = urlJoin(folder, `${filename}.tables.json`);
    let sync = false;

    if (await GBUtil.exists(tablesFile)) {
      const minBoot = GBServer.globals.minBoot;

      const tableDef = JSON.parse(await fs.readFile(tablesFile, 'utf8')) as any;

      const getTypeBasedOnCondition = (t, size) => {
        if (1) {
          switch (t) {
            case 'string':
              return `varchar(${size})`;
            case 'guid':
              return 'UUID';
            case 'key':
              return `varchar(${size})`;
            case 'number':
              return 'BIGINT';
            case 'integer':
              return 'INTEGER';
            case 'double':
              return 'FLOAT';
            case 'float':
              return 'FLOAT';
            case 'date':
              return 'DATE';
            case 'boolean':
              return 'BOOLEAN';
            default:
              return { type: 'TABLE', name: t };
          }
        } else {
          switch (t) {
            case 'string':
              return { key: 'STRING' };
            case 'guid':
              return { key: 'UUID' };
            case 'key':
              return { key: 'STRING' }; // Assuming key is a string data type
            case 'number':
              return { key: 'BIGINT' };
            case 'integer':
              return { key: 'INTEGER' };
            case 'double':
              return { key: 'FLOAT' };
            case 'float':
              return { key: 'FLOAT' };
            case 'date':
              return { key: 'DATE' };
            case 'boolean':
              return { key: 'BOOLEAN' };
            default:
              return { key: 'TABLE', name: t };
          }
        }
      };

      const associations = [];

      const shouldSync = min.core.getParam<boolean>(min.instance, 'Synchronize Database', false);

      tableDef.forEach(async t => {
        const tableName = t.name.trim().toLowerCase ();

        // Determines autorelationship.

        Object.keys(t.fields).forEach(key => {
          let obj = t.fields[key];
          obj.type = getTypeBasedOnCondition(obj.type, obj.size);
          if (obj.type.key === 'TABLE') {
            obj.type.key = 'BIGINT';
            associations.push({ from: tableName, to: obj.type.name });
          }
        });

        // Custom connection for TABLE.

        const connectionName = t.connection;
        let con = min[connectionName];

        if (!con) {
          GBLogEx.debug(min, `Invalid connection specified: ${min.bot} ${tableName} ${connectionName}.`);
        } else {

          // Field checking, syncs if there is any difference.

          const seq = con ? con : minBoot.core.sequelize;

          if (seq) {
            const model = seq.models[tableName];
            if (model) {

              // Except Id, checks if has same number of fields.

              let equals = 0;
              Object.keys(t.fields).forEach(key => {
                let obj1 = t.fields[key];
                let obj2 = model['fieldRawAttributesMap'][key];

                if (key !== 'id') {
                  if (obj1 && obj2) {
                    equals++;
                  }
                }
              });

              if (equals != Object.keys(t.fields).length) {
                sync = true;
              }
            }

            seq.define(tableName, t.fields);

            // New table checking, if needs sync.
            let tables;
            const dialect = con.dialect.name;

            tables = await GBUtil.listTables(dialect, seq);

            let found = false;
            tables.forEach(storageTable => {
              if (storageTable['table_name'] === tableName) {
                found = true;
              }
            });

            sync = sync ? sync : !found;

            // Do not erase tables in case of an error in collection retrieval.

            if (tables.length === 0) {
              sync = false;
            }

            associations.forEach(e => {
              const from = seq.models[e.from];
              const to = seq.models[e.to];

              try {
                to.hasMany(from);
              } catch (error) {
                throw new Error(
                  `Invalid relationship in ${mainName}: from ${e.from} to ${e.to} (${min.botId})... ${error.message}`
                );
              }
            });

            if (sync && shouldSync) {
              GBLogEx.info(min, `Syncing changes for TABLE ${connectionName} ${tableName} keyword (${min.botId})...`);

              await seq.sync({
                alter: true,
                force: false // Keep it false due to data loss danger.
              });
              GBLogEx.info(min, `Done sync for ${min.botId} ${connectionName} ${tableName} storage table...`);
            }
          }
        }
      });
    }
  }

  public async translateBASIC(mainName, filename: any, min: GBMinInstance) {

    // Converts General Bots BASIC into regular VBS

    let basicCode: string = await fs.readFile(filename, 'utf8');
    basicCode = GBVMService.normalizeQuotes(basicCode);

    // Pre process SET SCHEDULE calls.

    const schedules = GBVMService.getSetScheduleKeywordArgs(basicCode);

    const s = new ScheduleServices();
    await s.deleteScheduleIfAny(min, mainName);

    let i = 1;
    await CollectionUtil.asyncForEach(schedules, async syntax => {
      if (s) {
        await s.createOrUpdateSchedule(min, syntax, `${mainName};${i++}`);
      }
    });

    basicCode = basicCode.replace(/^\s*SET SCHEDULE (.*)/gim, '');

    // Process INCLUDE keyword to include another
    // dialog inside the dialog.

    let include = null;
    do {
      include = /^include\b(.*)$/gim.exec(basicCode);

      if (include) {
        let includeName = include[1].trim();
        includeName = path.join(path.dirname(filename), includeName);
        includeName = includeName.substr(0, includeName.lastIndexOf('.')) + '.vbs';

        // To use include, two /publish will be necessary (for now)
        // because of alphabet order may raise not found errors.

        let includeCode: string = await fs.readFile(includeName, 'utf8');
        basicCode = basicCode.replace(/^include\b.*$/gim, includeCode);
      }
    } while (include);

    let { code, map, metadata, tasks } = await this.convert(filename, mainName, basicCode);

    // Generates function JSON metadata to be used later.

    const jsonFile = `${filename}.json`;
    await fs.writeFile(jsonFile, JSON.stringify(metadata));

    const mapFile = `${filename}.map`;
    await fs.writeFile(mapFile, JSON.stringify(map));

    // Execute off-line code tasks

    await this.executeTasks(min, tasks);

    // Run JS into the GB context.

    const jsfile: string = `${filename}.js`;

    const template = (await fs.readFile('./vm-inject.js')).toString();
    code = template.replace('//##INJECTED_CODE_HERE', code);
    code = code.replace('//##INJECTED_HEADER', `port=${GBVMService.API_PORT}; botId='${min.botId}';`);

    code = ji.default(code, '  ');

    await fs.writeFile(jsfile, code);

  }

  private async executeTasks(min, tasks) {
    for (let i = 0; i < tasks.length; i++) {
      const task = tasks[i];

      if (task.kind === 'writeTableDefinition') {
        // Creates an empty object that will receive Sequelize fields.

        const tablesFile = `${task.file}.tables.json`;
        await fs.writeFile(tablesFile, JSON.stringify(task.tables));
      }
    }
  }

  public static getMethodNameFromVBSFilename(filename: string) {
    let mainName = filename.replace(/\s*|\-/gim, '').split('.')[0];
    return mainName.toLowerCase();
  }

  public static getSetScheduleKeywordArgs(code) {
    if (!code) return [];

    const lines = code.split(/\n/);
    const results = [];

    lines.forEach(line => {
      if (line.trim()) {
        const keyword = /^\s*SET SCHEDULE (.*)/gi;
        let result: any = keyword.exec(line);
        if (result) {
          result = result[1].replace(/\`|\"|\'/, '');
          result = result.trim();
          results.push(result);
        }
      }
    });

    return results;
  }

  private async getTextFromWord(folder: string, filename: string) {
    return new Promise<string>(async (resolve, reject) => {
      const filePath = urlJoin(folder, filename);
      textract.fromFileWithPath(filePath, { preserveLineBreaks: true }, async (error, text) => {
        if (error) {
          if (error.message.startsWith('File not correctly recognized as zip file')) {
            text = await fs.readFile(filePath, 'utf8');
          } else {
            reject(error);
          }
        }

        resolve(text);
      });
    });
  }

  public static normalizeQuotes(text: any) {
    text = text.replace(/\"/gm, '`');
    text = text.replace(/\¨/gm, '`');
    text = text.replace(/\“/gm, '`');
    text = text.replace(/\”/gm, '`');
    text = text.replace(/\‘/gm, "'");
    text = text.replace(/\’/gm, "'");

    return text;
  }

  public static getMetadata(mainName: string, propertiesText: string[][], description: string) {
    let properties = {};
    if (!propertiesText || !description) {
      return {};
    }

    const getType = (asClause: string) => {
      asClause = asClause.trim().toUpperCase();

      if (asClause.indexOf('STRING') !== -1) {
        return 'string';
      } else if (asClause.indexOf('OBJECT') !== -1) {
        return 'object';
      } else if (asClause.indexOf('INTEGER') !== -1 || asClause.indexOf('NUMBER') !== -1) {
        return 'number';
      } else {
        return 'enum';
      }
    };

    for (let i = 0; i < propertiesText.length; i++) {
      const propertiesExp = propertiesText[i];
      const t = getType(propertiesExp[2]);
      let element;
      const description = propertiesExp[4]?.trim();

      if (t === 'enum') {
        const list = propertiesExp[2] as any;
        element = z.enum(list.split(','));
      } else if (t === 'string') {
        element = z.string({ description: description });
      } else if (t === 'object') {
        element = z.string({ description: description }); // Assuming 'object' is represented as a string here
      } else if (t === 'number') {
        element = z.number({ description: description });
      } else {
        GBLog.warn(`Element type invalid specified on .docx: ${propertiesExp[0]}`);
      }

      element['type'] = t;
      properties[propertiesExp[1].trim()] = element;
    }

    const json = {
      type: 'function',
      function: {
        name: mainName,
        description: description ? description : '',
        schema: zodToJsonSchema(z.object(properties))
      },
      arguments: propertiesText.reduce((acc, prop) => {
        acc[prop[1].trim()] = prop[3]?.trim(); // Assuming value is in the 3rd index
        return acc;
      }, {})
    };

    return json;
  }

  public async parseField(line) {
    let required = line.indexOf('*') !== -1;
    let unique = /\bunique\b/gi.test(line);
    let primaryKey = /\bkey\b/gi.test(line);
    let autoIncrement = /\bauto\b/gi.test(line);

    if (primaryKey) {
      autoIncrement = true;
      unique = true;
      required = true;
    }

    line = line.replace('*', '');

    const fieldRegExp = /^\s*(\w+)\s*(\w+)(?:\((.*)\))?/gim;

    let reg = fieldRegExp.exec(line);
    const name = reg[1].toLowerCase();
    const t = reg[2];

    let definition = {
      allowNull: !required,
      unique: unique,
      primaryKey: primaryKey,
      autoIncrement: autoIncrement
    };
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
    let talk = null;
    let systemPrompt = null;
    let connection = null;
    const tasks = [];
    let fields = {};
    let tables = [];

    const outputLines = [];
    let emmitIndex = 1;
    for (let i = 1; i <= lines.length; i++) {
      let line = lines[i - 1];

      // Remove lines before statements.

      line = line.replace(/^\s*\d+\s*/gi, '');

      if (!table && !talk && !systemPrompt) {
        for (let j = 0; j < keywords.length; j++) {
          line = line.replace(keywords[j][0], keywords[j][1]); // TODO: Investigate delay here.
        }
      }

      // Pre-process "off-line" static KEYWORDS.

      let emmit = true;
      const params = /^\s*PARAM\s*(.*)\s*AS\s*(.*)\s*LIKE\s*(.*)\s*DESCRIPTION\s*(.*)/gim;
      const param = params.exec(line);
      if (param) {
        properties.push(param);
        emmit = false;
      }

      const descriptionKeyword = /^\s*DESCRIPTION\s(.*)/gim;
      let descriptionReg = descriptionKeyword.exec(line);
      if (descriptionReg) {
        description = descriptionReg[1];
        emmit = false;
      }

      const endSystemPromptKeyword = /^\s*END SYSTEM PROMPT\s*/gim;
      let endSystemPromptReg = endSystemPromptKeyword.exec(line);
      if (endSystemPromptReg && systemPrompt) {
        line = systemPrompt + '`})';

        systemPrompt = null;
        emmit = true;
      }

      const endTalkKeyword = /^\s*END TALK\s*/gim;
      let endTalkReg = endTalkKeyword.exec(line);
      if (endTalkReg && talk) {
        line = talk + '`})';

        talk = null;
        emmit = true;
      }

      const endTableKeyword = /^\s*END TABLE\s*/gim;
      let endTableReg = endTableKeyword.exec(line);
      if (endTableReg && table) {
        tables.push({
          name: table,
          fields: fields,
          connection: connection
        });

        fields = {};
        table = null;
        connection = null;
        emmit = false;
      }

      // Inside BEGIN TALK

      if (talk) {
        talk += line + '\\n';
        emmit = false;
      }

      // Inside BEGIN SYSTEM PROMPT

      if (systemPrompt) {
        systemPrompt += line + '\\n';
        emmit = false;
      }

      // Inside BEGIN/END table pair containing FIELDS.

      if (table && line.trim() !== '') {
        const field = await this.parseField(line);
        fields[field['name']] = field.definition;
        emmit = false;
      }

      const tableKeyword = /^\s*TABLE\s*(.*)\s*ON\s*(.*)/gim;
      let tableReg = tableKeyword.exec(line);
      if (tableReg && !table) {
        table = tableReg[1];
        connection = tableReg[2];
        emmit = false;
      }

      const talkKeyword = /^\s*BEGIN TALK\s*/gim;
      let talkReg = talkKeyword.exec(line);
      if (talkReg && !talk) {
        talk = 'await dk.talk ({pid: pid, text: `';
        emmit = false;
      }

      const systemPromptKeyword = /^\s*BEGIN SYSTEM PROMPT\s*/gim;
      let systemPromptReg = systemPromptKeyword.exec(line);
      if (systemPromptReg && !systemPrompt) {
        systemPrompt = 'await sys.setSystemPrompt ({pid: pid, text: `';
        emmit = false;
      }

      //  Add additional lines returned from replacement.

      let add = emmit ? line.split(/\r\n|\r|\n/).length : 0;
      current = current + (add ? add : 0);

      if (emmit) {
        emmitIndex++;
        map[emmitIndex] = current;
        outputLines[emmitIndex - 1] = line;
      }
    }

    if (tables) {
      tasks.push({
        kind: 'writeTableDefinition',
        file: filename,
        tables
      });
    }

    code = `${outputLines.join('\n')}\n`;

    let metadata = GBVMService.getMetadata(mainName, properties, description);

    return { code, map, metadata, tasks, systemPrompt };
  }

  /**
   * Executes the converted JavaScript from BASIC code inside execution context.
   */
  public static async callVM(text: string, min: GBMinInstance, step, pid, debug: boolean = false, params = []) {
    // Creates a class DialogKeywords which is the *this* pointer
    // in BASIC.

    const sandbox = {};
    const contentLocale = min.core.getParam<string>(
      min.instance,
      'Default Content Language',
      GBConfigService.get('DEFAULT_CONTENT_LANGUAGE')
    );

    let variables = {};

    // These variables will be automatically be available as normal BASIC variables.

    try {
      variables['aadToken'] = await (min.adminService as any)['acquireElevatedToken'](min.instance.instanceId, false);
    } catch (error) {
      variables['aadToken'] = 'ERROR: Configure /setupSecurity before using aadToken variable.';
    }

    // Adds all .gbot params as variables.

    const gbotConfig = JSON.parse(min.instance.params);
    let keys = Object.keys(gbotConfig);
    for (let j = 0; j < keys.length; j++) {
      const v = keys[j].replace(/\s/gi, '');
      variables[v] = gbotConfig[keys[j]];
    }

    // Auto-NLP generates BASIC variables related to entities.

    if (step?.context?.activity.originalText && min['nerEngine']) {
      const result = await min['nerEngine'].process(step.context.activity.originalText);

      for (let i = 0; i < result.entities.length; i++) {
        const v = result.entities[i];
        const variableName = `${v.entity}`;
        variables[variableName] = v.option ? v.option : v.sourceText;
      }
    }

    // Adds params as variables to be added later as global objects.

    keys = Object.keys(params);
    for (let j = 0; j < keys.length; j++) {
      variables[keys[j]] = params[keys[j]];
    }

    const botId = min.botId;
    const packagePath = GBUtil.getGBAIPath(min.botId, `gbdialog`);
    const gbdialogPath = urlJoin(process.cwd(), 'work', packagePath);
    const scriptFilePath = urlJoin(gbdialogPath, `${text}.js`);

    let code = min.sandBoxMap[text];
    const channel = step?.context ? step.context.activity.channelId : 'web';

    const dk = new DialogKeywords();
    const sys = new SystemKeywords();
    await dk.setFilter({ pid: pid, value: null });

    // Find all tokens in .gbot Config.

    const strFind = ' Client ID';
    const tokens = await min.core['findParam'](min.instance, strFind);
    let tokensList = [];
    await CollectionUtil.asyncForEach(tokens, async t => {
      const tokenName = t.replace(strFind, '');
      tokensList.push(tokenName);
    });

    sandbox['tokens'] = tokensList.join(',');
    sandbox['variables'] = variables;
    sandbox['id'] = sys.getRandomId();
    sandbox['username'] = await dk.userName({ pid });
    sandbox['mobile'] = await dk.userMobile({ pid });
    sandbox['from'] = await dk.userMobile({ pid });
    sandbox['ENTER'] = String.fromCharCode(13);
    sandbox['headers'] = {};
    sandbox['httpUsername'] = '';
    sandbox['httpPs'] = '';
    sandbox['pid'] = pid;
    sandbox['contentLocale'] = contentLocale;
    sandbox['callTimeout'] = 60 * 60 * 24 * 1000;
    sandbox['channel'] = channel;
    sandbox['today'] = await dk.getToday({ pid });
    sandbox['now'] = await dk.getNow({ pid });
    sandbox['returnValue'] = null;
    let result;

    try {
      if (!GBConfigService.get('GBVM')) {
        return await (async () => {
          return await new Promise((resolve) => {
            sandbox['resolve'] = resolve;
            // TODO: #411 sandbox['reject'] = reject;
            sandbox['reject'] = () => { };

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
            const s = new VMScript(code, { filename: scriptFilePath });
            result = vm1.run(s);
          });
        })();
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
          // debuggerport: GBVMService.DEBUGGER_PORT,
          botId: botId,
          cpu: 100,
          memory: 50000,
          time: 60 * 60 * 24 * 14,
          cwd: gbdialogPath,
          script: runnerPath
        });

        result = await run(code, Object.assign(sandbox, { filename: scriptFilePath }));
      }
    } catch (error) {
      throw new Error(`BASIC RUNTIME ERR: ${error.message ? error.message : error}\n Stack:${error.stack}`);
    }
  }

  public static createProcessInfo(
    user: GuaribasUser,
    min: GBMinInstance,
    channel: any,
    executable: string,
    step = null
  ) {
    const pid = GBAdminService.getNumberIdentifier();
    GBServer.globals.processes[pid] = {
      pid: pid,
      userId: user ? user.userId : 0,
      instanceId: min.instance.instanceId,
      channel: channel,
      roles: 'everyone',
      step: step,
      executable: executable
    };
    return pid;
  }
}
