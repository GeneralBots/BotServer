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

/**
 * @fileoverview Knowledge base services and logic.
 */

import Path from 'path';
import Fs from 'fs';
import urlJoin from 'url-join';
import path from 'path';
import asyncPromise from 'async-promises';
import walkPromise from 'walk-promise';
import { SearchClient } from '@azure/search-documents';
import Excel from 'exceljs';
import getSlug from 'speakingurl';
import { GBServer } from '../../../src/app.js';
import {
  GBDialogStep,
  GBLog,
  GBMinInstance,
  IGBConversationalService,
  IGBCoreService,
  IGBInstance,
  IGBKBService
} from 'botlib';
import { CollectionUtil } from 'pragmatismo-io-framework';
import { Op } from 'sequelize';
import { Sequelize } from 'sequelize-typescript';
import { AzureDeployerService } from '../../azuredeployer.gbapp/services/AzureDeployerService.js';
import { GuaribasPackage } from '../../core.gbapp/models/GBModel.js';
import { GBDeployer } from '../../core.gbapp/services/GBDeployer.js';
import { CSService } from '../../customer-satisfaction.gbapp/services/CSService.js';
import { GuaribasAnswer, GuaribasQuestion, GuaribasSubject } from '../models/index.js';
import { GBConfigService } from './../../core.gbapp/services/GBConfigService.js';
import { parse } from 'node-html-parser';
import textract from 'textract';
import pdf from 'pdf-extraction';
import { GBSSR } from '../../core.gbapp/services/GBSSR.js';
import { GBLogEx } from '../../core.gbapp/services/GBLogEx.js';
import mammoth from 'mammoth';
import { url } from 'inspector';
import { min } from 'lodash';
import { GBAdminService } from '../../admin.gbapp/services/GBAdminService.js';
import { text } from 'body-parser';
import { GBVMService } from '../../basic.gblib/services/GBVMService.js';

/**
 * Result for quey on KB data.
 */
export class KBServiceSearchResults {
  public answer: GuaribasAnswer;
  public questionId: number;
}

/**
 * All services related to knowledge base management.
 */
export class KBService implements IGBKBService {
  public sequelize: Sequelize;

  constructor(sequelize: Sequelize) {
    this.sequelize = sequelize;
  }

  public static getFormattedSubjectItems(subjects: GuaribasSubject[]) {
    if (subjects === null) {
      return '';
    }
    const out = [];
    subjects.forEach(subject => {
      out.push(subject.title);
    });

    return out.join(', ');
  }

  public static getSubjectItemsSeparatedBySpaces(subjects: GuaribasSubject[]) {
    const out = [];
    if (subjects === undefined) {
      return '';
    }
    subjects.forEach(subject => {
      out.push(subject.internalId);
    });

    return out.join(' ');
  }

  public async getAnswerTextByMediaName(instanceId: number, answerMediaName: string): Promise<string> {
    const answer = await GuaribasAnswer.findOne({
      where: {
        instanceId: instanceId,
        media: answerMediaName
      }
    });

    return answer != undefined ? answer.content : null;
  }

  public async getQuestionById(instanceId: number, questionId: number): Promise<GuaribasQuestion> {
    return GuaribasQuestion.findOne({
      where: {
        instanceId: instanceId,
        questionId: questionId
      }
    });
  }

  public async getAnswerById(instanceId: number, answerId: number): Promise<GuaribasAnswer> {
    return await GuaribasAnswer.findOne({
      where: {
        instanceId: instanceId,
        answerId: answerId
      }
    });
  }

  /**
   * Returns a question object given a SEO friendly URL.
   */
  public async getQuestionIdFromURL(core: IGBCoreService, url: string) {
    // Extracts questionId from URL.

    const id = url.substr(url.lastIndexOf('-') + 1);

    // Extracts botId from URL.

    let path = /(http[s]?:\/\/)?([^\/\s]+\/)(.*)/gi;
    const botId = url.replace(path, ($0, $1, $2, $3) => {
      return $3.substr($3.indexOf('/'));
    });

    // Finds the associated question.

    const instance = await core.loadInstanceByBotId(botId);
    const question = await GuaribasQuestion.findAll({
      where: {
        instanceId: instance.instanceId,
        questionId: id
      }
    });

    return question;
  }
  public static async getQuestionsNER(instanceId: number) {
    const where = {
      instanceId: instanceId,
      content: { [Op.like]: `%(%` }
    };

    const questions = await GuaribasQuestion.findAll({
      where: where
    });

    return questions;
  }

  public async getQuestionsSEO(instanceId: number) {
    const questions = await GuaribasQuestion.findAll({
      where: {
        instanceId: instanceId
      }
    });

    let output = [];
    for (let i = 0; i < questions.length; i++) {
      const answer = questions[i];
      const text = getSlug(answer.content);
      let url = `${text}-${i}`;
      output.push(url);
    }

    return output;
  }

  public async getDocs(instanceId: number) {
    return await GuaribasAnswer.findAll({
      where: {
        instanceId: instanceId,
        format: '.docx'
      }
    });
  }

  public async getAnswerByText(instanceId: number, text: string, from: string = null): Promise<any> {
    text = text.trim();

    const service = new CSService();
    let question = await service.getQuestionFromAlternateText(instanceId, text);

    if (!question) {
      const where = {
        instanceId: instanceId,
        content: { [Op.like]: `%^[\w.]+${text}^[\w.]+%` }
      };

      if (from) {
        where['from'] = from;
      }
      question = await GuaribasQuestion.findOne({
        where: where
      });
    }
    if (!question) {
      let where = {
        instanceId: instanceId,
        content: { [Op.eq]: `${text}` }
      };
      question = await GuaribasQuestion.findOne({
        where: where
      });
    }

    if (question !== null) {
      const answer = await GuaribasAnswer.findOne({
        where: {
          instanceId: instanceId,
          answerId: question.answerId
        }
      });

      return { question: question, answer: answer };
    }

    return undefined;
  }

  public async addAnswer(obj: GuaribasAnswer): Promise<GuaribasAnswer> {
    return await GuaribasAnswer.create(obj);
  }

  public async ask(
    instance: IGBInstance,
    query: string,
    searchScore: number,
    subjects: GuaribasSubject[]
  ): Promise<KBServiceSearchResults> {
    // Builds search query.

    query = query.toLowerCase();
    query = query.replace('?', ' ');
    query = query.replace('!', ' ');
    query = query.replace('.', ' ');
    query = query.replace('/', ' ');
    query = query.replace('\\', ' ');
    query = query.replace('\r\n', ' ');

    // Try simple search first.

    const data = await this.getAnswerByText(instance.instanceId, query.trim());
    if (data) {
      GBLog.info(`Simple SEARCH called.`);
      return { answer: data.answer, questionId: data.question.questionId };
    }

    if (subjects !== null) {
      const text = KBService.getSubjectItemsSeparatedBySpaces(subjects);
      if (text !== null) {
        query = `${query} ${text}`;
      }
    }

    // No direct match found, so Search is used.

    if (instance.searchKey !== null && GBConfigService.get('STORAGE_DIALECT') === 'mssql') {
      interface SearchResults {
        instanceId: number;
        questionId: number;
        answerId: number;
        content: string;
        subject1: string;
        subject2: string;
        subject3: string;
        subject4: string;
      }

      const client = new SearchClient<any>('https://' + instance.searchHost, 'azuresql-index', {
        key: instance.searchKey
      } as any);

      const results = await client.search(query, {
        filter: `instanceId eq ${instance.instanceId} and skipIndex eq false`,
        searchFields: ['content', 'subject1', 'subject2', 'subject3', 'subject4'],
        select: ['instanceId', 'questionId', 'answerId'],
        skip: 0,
        top: 1
      });

      let returnedScore = 0;

      // Searches via Search (Azure Search).

      let found = false;
      for await (const result of results.results) {
        found = true;
        returnedScore = result.score;
        if (returnedScore >= searchScore) {
          const value = await this.getAnswerById(instance.instanceId, result.document.answerId);
          if (value !== null) {
            GBLog.info(`SEARCH WILL BE USED with score: ${returnedScore} > required (searchScore): ${searchScore}`);

            return { answer: value, questionId: result.document.questionId };
          } else {
            GBLog.info(
              `Index problem. SEARCH WILL NOT be used as answerId ${result.document.answerId} was not found in database,
                returnedScore: ${returnedScore} < required (searchScore): ${searchScore}`
            );

            return { answer: undefined, questionId: 0 };
          }
        } else {
          GBLog.info(
            `SEARCH called but returned LOW level score,
              returnedScore: ${returnedScore} < required (searchScore): ${searchScore}`
          );

          return { answer: undefined, questionId: 0 };
        }
      }

      return { answer: undefined, questionId: 0 };
    }
  }

  public async getSubjectItems(instanceId: number, parentId: number): Promise<GuaribasSubject[]> {
    const where = { parentSubjectId: parentId, instanceId: instanceId };

    return GuaribasSubject.findAll({
      where: where
    });
  }

  public async getFaqBySubjectArray(instanceId: number, from: string, subjects: any): Promise<GuaribasQuestion[]> {
    if (subjects) {
      const where = {
        from: from,
        // tslint:disable-next-line: no-null-keyword
        subject1: null,
        // tslint:disable-next-line: no-null-keyword
        subject2: null,
        // tslint:disable-next-line: no-null-keyword
        subject3: null,
        // tslint:disable-next-line: no-null-keyword
        subject4: null,
        // tslint:disable-next-line: no-null-keyword
        instanceId: instanceId
      };

      if (subjects[0] && subjects[0].internalId) {
        where.subject1 = subjects[0].internalId;
      }

      if (subjects[1] && subjects[1].internalId) {
        where.subject2 = subjects[1].internalId;
      }

      if (subjects[2] && subjects[2].internalId) {
        where.subject3 = subjects[2].internalId;
      }

      if (subjects[3] && subjects[3].internalId) {
        where.subject4 = subjects[3].internalId;
      }

      return await GuaribasQuestion.findAll({
        where: where
      });
    } else {
      return await GuaribasQuestion.findAll({
        where: { from: from, instanceId: instanceId }
      });
    }
  }

  public static async getGroupReplies(instanceId: number): Promise<GuaribasQuestion[]> {
    return await GuaribasQuestion.findAll({
      where: { from: 'group', instanceId: instanceId }
    });
  }

  public async importKbTabularFile(
    filePath: string,
    min: GBMinInstance,
    packageId: number
  ): Promise<GuaribasQuestion[]> {
    GBLog.info(`Now reading file ${filePath}...`);
    const workbook = new Excel.Workbook();
    const data = await workbook.xlsx.readFile(filePath);

    let lastQuestionId: number;
    let lastAnswer: GuaribasAnswer;

    // Finds a valid worksheet because Excel returns empty slots
    // when loading worksheets collection.

    let worksheet: any;
    for (let t = 0; t < data.worksheets.length; t++) {
      worksheet = data.worksheets[t];
      if (worksheet) {
        break;
      }
    }

    const rows = worksheet._rows;
    const answers = [];
    const questions = [];

    GBLog.info(`Processing ${rows.length} rows from tabular file ${filePath}...`);
    await asyncPromise.eachSeries(rows, async line => {
      // Skips the first line.

      if (
        line != undefined &&
        line._cells[0] !== undefined &&
        line._cells[1] !== undefined &&
        line._cells[2] !== undefined &&
        line._cells[3] !== undefined &&
        line._cells[4] !== undefined
      ) {
        // Extracts values from columns in the current line.

        const subjectsText = line._cells[0].text;
        const from = line._cells[1].text;
        const to = line._cells[2].text;
        const question = line._cells[3].text.trim();
        let answer = line._cells[4].text.trim();

        if (
          !(subjectsText === 'subjects' && from === 'from') &&
          answer !== null &&
          question !== null &&
          answer !== '' &&
          question !== ''
        ) {
          let format = '.txt';

          // Extracts answer from external media if any.

          let media = null;

          if (typeof answer !== 'string') {
            GBLog.info(`[GBImporter] Answer is NULL related to Question '${question}'.`);
            answer =
              'Existe um problema na base de conhecimento. Fui treinado para entender sua pergunta, avise a quem me criou que a resposta não foi informada para esta pergunta.';
          } else if (answer.indexOf('.md') > -1 || answer.indexOf('.docx') > -1) {
            const mediaFilename = urlJoin(path.dirname(filePath), '..', 'articles', answer);
            if (Fs.existsSync(mediaFilename)) {
              // Tries to load .docx file from Articles folder.

              if (answer.indexOf('.docx') > -1) {
                answer = await this.getTextFromFile(filePath);
              } else {
                // Loads normally markdown file.

                answer = Fs.readFileSync(mediaFilename, 'utf8');
              }
              format = '.md';
              media = path.basename(mediaFilename);
            } else {
              if (answer.indexOf('.md') > -1) {
                GBLog.info(`[GBImporter] File not found: ${mediaFilename}.`);
                answer = '';
              }
            }
          }

          // Processes subjects hierarchy splitting by dots.

          const subjectArray = subjectsText.split('.');
          let subject1: string;
          let subject2: string;
          let subject3: string;
          let subject4: string;
          let indexer = 0;

          subjectArray.forEach(element => {
            if (indexer === 0) {
              subject1 = subjectArray[indexer].substring(0, 63);
            } else if (indexer === 1) {
              subject2 = subjectArray[indexer].substring(0, 63);
            } else if (indexer === 2) {
              subject3 = subjectArray[indexer].substring(0, 63);
            } else if (indexer === 3) {
              subject4 = subjectArray[indexer].substring(0, 63);
            }
            indexer++;
          });

          // Skips blank answers.

          if (answer && answer.trim() === '') {
            return false;
          }

          // In case  of code cell, compiles it and associate with the answer.

          answer = GBVMService.normalizeQuotes(answer);
          const isBasic = answer.toLowerCase().startsWith('/basic');
          if (/TALK\s*\".*\"/gi.test(answer) || isBasic) {
            const code = isBasic ? answer.substr(6) : answer;
            const gbaiName = `${min.instance.botId}.gbai`;
            const gbdialogName = `${min.instance.botId}.gbdialog`;
            const scriptName = `tmp${GBAdminService.getRndReadableIdentifier()}.docx`;
            const localName = Path.join('work', gbaiName, gbdialogName, `${scriptName}`);
            Fs.writeFileSync(localName, code, { encoding: null });
            answer = scriptName;

            const vm = new GBVMService();
            await vm.loadDialog(Path.basename(localName), Path.dirname(localName), min);
          }

          // Now with all the data ready, creates entities in the store.

          const answer1 = {
            instanceId: min.instance.instanceId,
            content: answer,
            format: format,
            media: media,
            packageId: packageId,
            prevId: lastQuestionId !== null ? lastQuestionId : 0
          };

          answers.push(answer1);

          const question1 = {
            from: from,
            to: to,
            subject1: subject1,
            subject2: subject2,
            subject3: subject3,
            subject4: subject4,
            content: question.replace(/["]+/g, ''),
            instanceId: min.instance.instanceId,
            skipIndex: question.charAt(0) === '"',
            packageId: packageId
          };
          questions.push(question1);

          // https://github.com/GeneralBots/BotServer/issues/312
          // if (lastAnswer !== undefined && lastQuestionId !== 0) {
          //   await lastAnswer.update({ nextId: lastQuestionId });
          // }
          // lastAnswer = answer1;
          // lastQuestionId = question1.questionId;

          return true;
        } else {
          // Skips the header.

          return undefined;
        }
      }
    });

    const answersCreated = await GuaribasAnswer.bulkCreate(answers);

    let i = 0;
    await CollectionUtil.asyncForEach(questions, async question => {
      question.answerId = answersCreated[i++].answerId;
    });

    return await GuaribasQuestion.bulkCreate(questions);
  }

  public async sendAnswer(min: GBMinInstance, channel: string, step: GBDialogStep, answer: GuaribasAnswer) {
    if (answer.content.endsWith('.mp4')) {
      await this.playVideo(min, min.conversationalService, step, answer, channel);
    } else if (
      answer.content.endsWith('.ppt') ||
      answer.content.endsWith('.pptx') ||
      answer.content.endsWith('.doc') ||
      answer.content.endsWith('.docx') ||
      answer.content.endsWith('.xls') ||
      answer.content.endsWith('.xlsx')
    ) {
      const doc = urlJoin(
        GBServer.globals.publicAddress,
        'kb',
        `${min.instance.botId}.gbai`,
        `${min.instance.botId}.gbkb`,
        'assets',
        answer.content
      );
      const url = `http://view.officeapps.live.com/op/view.aspx?src=${doc}`;
      await this.playUrl(min, min.conversationalService, step, url, channel);
    } else if (answer.content.endsWith('.pdf')) {
      const url = urlJoin('kb', `${min.instance.botId}.gbai`, `${min.instance.botId}.gbkb`, 'assets', answer.content);
      await this.playUrl(min, min.conversationalService, step, url, channel);
    } else if (answer.format === '.md') {
      await min.conversationalService['playMarkdown'](min, answer.content, channel, step, min.conversationalService);
    } else if (answer.content.endsWith('.ogg') && process.env.AUDIO_DISABLED !== 'true') {
      await this.playAudio(min, answer, channel, step, min.conversationalService);
    } else {
      await min.conversationalService.sendText(min, step, answer.content);
      await min.conversationalService.sendEvent(min, step, 'stop', undefined);
    }
  }

  public async importKbPackage(
    min: GBMinInstance,
    localPath: string,
    packageStorage: GuaribasPackage,
    instance: IGBInstance
  ): Promise<any> {
    // Imports subjects tree into database and return it.

    const subjectFile = urlJoin(localPath, 'subjects.json');
    const menuFile = urlJoin(localPath, 'menu.xlsx');

    if (Fs.existsSync(subjectFile) || Fs.existsSync(menuFile)) {
      await this.importSubjectFile(packageStorage.packageId, subjectFile, menuFile, instance);
    }

    // Import tabular files in the tabular directory.

    await this.importKbTabularDirectory(localPath, min, packageStorage.packageId);

    // Import remaining .md files in articles directory.

    await this.importRemainingArticles(localPath, instance, packageStorage.packageId);

    // Import docs files in .docx directory.

    return await this.importDocs(min, localPath, instance, packageStorage.packageId);
  }

  /**
   * Import all .md files in articles folder that has not been referenced by tabular files.
   */
  public async importRemainingArticles(localPath: string, instance: IGBInstance, packageId: number): Promise<any> {
    const files = await walkPromise(urlJoin(localPath, 'articles'));
    const data = { questions: [], answers: [] };

    await CollectionUtil.asyncForEach(files, async file => {
      if (file !== null && file.name.endsWith('.md')) {
        let content = await this.getAnswerTextByMediaName(instance.instanceId, file.name);

        if (content === null) {
          const fullFilename = urlJoin(file.root, file.name);
          content = Fs.readFileSync(fullFilename, 'utf-8');

          await GuaribasAnswer.create(<GuaribasAnswer>{
            instanceId: instance.instanceId,
            content: content,
            format: '.md',
            media: file.name,
            packageId: packageId,
            prevId: 0 // https://github.com/GeneralBots/BotServer/issues/312
          });
        }
      } else if (file !== null && file.name.endsWith('.docx')) {
        const gbaiName = `${instance.botId}.gbai`;
        const gbkbName = `${instance.botId}.gbkb`;
        const localName = Path.join('work', gbaiName, gbkbName, 'articles', file.name);
        const buffer = Fs.readFileSync(localName, { encoding: null });
        var options = {
          buffer: buffer,
          convertImage: async image => {
            const localName = Path.join(
              'work',
              gbaiName,
              'cache',
              `img-docx${GBAdminService.getRndReadableIdentifier()}.png`
            );
            const url = urlJoin(GBServer.globals.publicAddress, instance.botId, 'cache', Path.basename(localName));
            const buffer = await image.read();
            Fs.writeFileSync(localName, buffer, { encoding: null });
            return { src: url };
          }
        };

        let state = 0;
        let previousState = state;
        const next = (root, el, data) => {
          // If it is root, change to the first item.

          if (el.parentNode == null) {
            el = el.firstChild;
          }
          let value = el.innerHTML;
          const isHeader = el => el.rawTagName.startsWith('h') && el.rawTagName.length === 2;

          // Handle questions from H* elements.

          if (state === 0) {
            const question = {
              from: 'document',
              to: '',
              subject1: '',
              subject2: '',
              subject3: '',
              subject4: '',
              content: value.replace(/["]+/g, ''),
              instanceId: instance.instanceId,
              skipIndex: 0,
              packageId: packageId
            };
            data.questions.push(question);
            previousState = state;
            state = 1;

            // Everything else is content for that Header.
          } else if (state === 1) {
            // If next element is null, the tree has been passed, so
            // finish the append of other elements between the last Header
            // and the end of the document.

            if (!el.nextSibling || isHeader(el.nextSibling)) {
              const answer = {
                instanceId: instance.instanceId,
                content: value,
                format: '.html',
                media: file.name,
                packageId: packageId,
                prevId: 0
              };

              data.answers.push(answer);

              state = 0;

              // Otherwise, just append content to insert later.
            } else {
              value += value;
            }
          }

          // Goes to the next node, as it is all same level nodes.

          if (el.nextSibling) {
            next(root, el.nextSibling, data);
          }
        };

        const html = await mammoth.convertToHtml(options);
        const root = parse(html.value);
        next(root, root, data);
      }

      // Persist to storage.

      const answersCreated = await GuaribasAnswer.bulkCreate(data.answers);
      let i = 0;
      await CollectionUtil.asyncForEach(data.questions, async question => {
        question.answerId = answersCreated[i++].answerId;
      });
      return await GuaribasQuestion.bulkCreate(data.questions);
    });
  }

  /**
   * Import all .docx files in reading comprehension folder.
   */
  public async importDocs(
    min: GBMinInstance,
    localPath: string,
    instance: IGBInstance,
    packageId: number
  ): Promise<any> {
    const files = await walkPromise(urlJoin(localPath, 'docs'));
    if (!files[0]) {
      GBLog.info(
        `[GBDeployer] docs folder not created yet in .gbkb. To use Reading Comprehension, create this folder at root and put a document to get read by the.`
      );
    } else {
      await CollectionUtil.asyncForEach(files, async file => {
        let content = null;
        let filePath = Path.join(file.root, file.name);
        if (file !== null) {
          if (file.name.endsWith('.docx')) {
            content = await this.getTextFromFile(filePath);
          } else if (file.name.endsWith('.pdf')) {
            const read = await pdf(Fs.readFileSync(filePath));
            content = read.text;
          }
        }

        if (content) {
          content = await min.conversationalService.translate(min, content, 'en');
          await GuaribasAnswer.create(<GuaribasAnswer>{
            instanceId: instance.instanceId,
            content: content,
            format: '.docx',
            media: file.name,
            packageId: packageId
          });
        }
      });
    }
  }

  public async importKbTabularDirectory(localPath: string, min: GBMinInstance, packageId: number): Promise<any> {
    const files = await walkPromise(localPath);

    await CollectionUtil.asyncForEach(files, async file => {
      if (file !== null && file.name.endsWith('.xlsx')) {
        return await this.importKbTabularFile(urlJoin(file.root, file.name), min, packageId);
      }
    });
  }

  public async importSubjectFile(
    packageId: number,
    filename: string,
    menuFile: string,
    instance: IGBInstance
  ): Promise<any> {
    let subjectsLoaded;
    if (menuFile) {
      // Loads menu.xlsx and finds worksheet.

      const workbook = new Excel.Workbook();
      const data = await workbook.xlsx.readFile(menuFile);
      let worksheet: any;
      for (let t = 0; t < data.worksheets.length; t++) {
        worksheet = data.worksheets[t];
        if (worksheet) {
          break;
        }
      }

      const MAX_LEVEL = 4; // Max column level to reach menu items in plan.
      // Iterates over all items.

      let rows = worksheet._rows;
      rows.length = 24;
      let lastLevel = 0;
      let subjects = { children: [] };
      let childrenNode = subjects.children;
      let activeObj = null;

      let activeChildrenGivenLevel = [childrenNode];

      await asyncPromise.eachSeries(rows, async row => {
        if (!row) return;
        let menu;

        // Detect menu level by skipping blank cells on left.
        
        let level;
        for (level = 0; level < MAX_LEVEL; level++) {
          const cell = row._cells[level];
          if (cell && cell.text) {
            menu = cell.text;
            break;
          }
        }

        // Tree hierarchy calculation.        

         if (level > lastLevel) {
          childrenNode = activeObj.children;
        } else if (level < lastLevel) {
          childrenNode = activeChildrenGivenLevel[level];
        }

        /// Keeps the record of last subroots for each level, to
        // changel levels greater than one (return to main menu),
        // can exists between leaf nodes and roots.

        activeChildrenGivenLevel[level] = childrenNode;

        // Insert the object into JSON.
        const description  = row._cells[level + 1]?row._cells[level + 1].text: null;
        activeObj = {
          title: menu,
          description: description,
          id: menu,
          children: []
        };
        activeChildrenGivenLevel[level].push(activeObj);

        lastLevel = level;
      });

      subjectsLoaded = subjects;
    } else {
      subjectsLoaded = JSON.parse(Fs.readFileSync(filename, 'utf8'));
    }

    const doIt = async (subjects: GuaribasSubject[], parentSubjectId: number) => {
      return asyncPromise.eachSeries(subjects, async item => {
        const value = await GuaribasSubject.create(<GuaribasSubject>{
          internalId: item.id,
          parentSubjectId: parentSubjectId,
          instanceId: instance.instanceId,
          from: item.from,
          to: item.to,
          title: item.title,
          description: item.description,
          packageId: packageId
        });

        if (item.children) {
          return doIt(item.children, value.subjectId);
        } else {
          return item;
        }
      });
    };

    return doIt(subjectsLoaded.children, undefined);
  }

  public async undeployKbFromStorage(instance: IGBInstance, deployer: GBDeployer, packageId: number) {
    await GuaribasQuestion.destroy({
      where: { instanceId: instance.instanceId, packageId: packageId }
    });
    await GuaribasAnswer.destroy({
      where: { instanceId: instance.instanceId, packageId: packageId }
    });
    await GuaribasSubject.destroy({
      where: { instanceId: instance.instanceId, packageId: packageId }
    });
    await this.undeployPackageFromStorage(instance, packageId);
  }

  public static async RefreshNER(min: GBMinInstance) {
    const questions = await KBService.getQuestionsNER(min.instance.instanceId);
    const contentLocale = min.core.getParam<string>(
      min.instance,
      'Default Content Language',
      GBConfigService.get('DEFAULT_CONTENT_LANGUAGE')
    );

    await CollectionUtil.asyncForEach(questions, async question => {
      const text = question.content;

      const categoryReg = /.*\((.*)\).*/gi.exec(text);
      const nameReg = /(\w+)\(.*\).*/gi.exec(text);

      if (categoryReg && nameReg) {
        let category = categoryReg[1];
        let name = nameReg[1];
        min['nerEngine'].addNamedEntityText(category, name, [contentLocale], [name]);
      }
    });
  }

  /**
   * Deploys a knowledge base to the storage using the .gbkb format.
   *
   * @param localPath Path to the .gbkb folder.
   */
  public async deployKb(core: IGBCoreService, deployer: GBDeployer, localPath: string, min: GBMinInstance) {
    const packageName = Path.basename(localPath);
    const instance = await core.loadInstanceByBotId(min.botId);
    GBLog.info(`[GBDeployer] Importing: ${localPath}`);
    const p = await deployer.deployPackageToStorage(instance.instanceId, packageName);
    await this.importKbPackage(min, localPath, p, instance);
    GBDeployer.mountGBKBAssets(packageName, min.botId, localPath);
    const service = await AzureDeployerService.createInstance(deployer);
    await deployer.rebuildIndex(instance, service.getKBSearchSchema(instance.searchIndex));

    min['groupCache'] = await KBService.getGroupReplies(instance.instanceId);
    await KBService.RefreshNER(min);

    GBLog.info(`[GBDeployer] Start Bot Server Side Rendering... ${localPath}`);
    const html = await GBSSR.getHTML(min);
    const path = Path.join(
      process.env.PWD,
      'work',
      `${min.instance.botId}.gbai`,
      `${min.instance.botId}.gbui`,
      'index.html'
    );
    GBLogEx.info(min, `[GBDeployer] Saving SSR HTML in ${path}.`);
    Fs.writeFileSync(path, html, 'utf8');

    GBLog.info(`[GBDeployer] Finished import of ${localPath}`);
  }

  private async playAudio(
    min: GBMinInstance,
    answer: GuaribasAnswer,
    channel: string,
    step: GBDialogStep,
    conversationalService: IGBConversationalService
  ) {
    conversationalService.sendAudio(min, step, answer.content);
  }

  private async playUrl(
    min,
    conversationalService: IGBConversationalService,
    step: GBDialogStep,
    url: string,
    channel: string
  ) {
    if (channel === 'whatsapp') {
      await min.conversationalService.sendFile(min, step, null, url, '');
    } else {
      await conversationalService.sendEvent(min, step, 'play', {
        playerType: 'url',
        data: url
      });
    }
  }

  private async playVideo(
    min,
    conversationalService: IGBConversationalService,
    step: GBDialogStep,
    answer: GuaribasAnswer,
    channel: string
  ) {
    if (channel === 'whatsapp') {
      await min.conversationalService.sendFile(min, step, null, answer.content, '');
    } else {
      await conversationalService.sendEvent(min, step, 'play', {
        playerType: 'video',
        data: urlJoin(`${min.instance.botId}.gbai`, `${min.instance.botId}.gbkb`, 'videos', answer.content)
      });
    }
  }

  private async undeployPackageFromStorage(instance: any, packageId: number) {
    await GuaribasPackage.destroy({
      where: { instanceId: instance.instanceId, packageId: packageId }
    });
  }

  public async readComprehension(instanceId: number, doc: string, question: string) {
    const url =
      `http://${process.env.GBMODELS_SERVER}/reading-comprehension` +
      new URLSearchParams({ question: question, key: process.env.GBMODELS_KEY });
    const form = new FormData();
    form.append('content', doc);
    const options = {
      body: form
    };
    GBLog.info(`[General Bots Models]: ReadComprehension for ${question}.`);
    return await fetch(url, options);
  }

  private async getTextFromFile(filename: string) {
    return new Promise<string>(async (resolve, reject) => {
      textract.fromFileWithPath(filename, { preserveLineBreaks: true }, (error, text) => {
        if (error) {
          reject(error);
        } else {
          resolve(text);
        }
      });
    });
  }
}
