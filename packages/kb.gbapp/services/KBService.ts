/*****************************************************************************\
|  █████  █████ ██    █ █████ █████   ████  ██      ████   █████ █████  ███ ® |
| ██      █     ███   █ █     ██  ██ ██  ██ ██      ██  █ ██   ██  █   █      |
| ██  ███ ████  █ ██  █ ████  █████  ██████ ██      ████   █   █   █    ██    |
| ██   ██ █     █  ██ █ █     ██  ██ ██  ██ ██      ██  █ ██   ██  █      █   |
|  █████  █████ █   ███ █████ ██  ██ ██  ██ █████   ████   █████   █   ███    |
|                                                                             |
| General Bots Copyright (c) pragmatismo.cloud. All rights reserved.         |
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
| "General Bots" is a registered trademark of pragmatismo.cloud.             |
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
import asyncPromise from 'async-promises';
import walkPromise from 'walk-promise';
import { SearchClient } from '@azure/search-documents';
import Excel from 'exceljs';
import getSlug from 'speakingurl';
import { GBServer } from '../../../src/app.js';
import { JSONLoader } from 'langchain/document_loaders/fs/json';
import { TextLoader } from 'langchain/document_loaders/fs/text';
import { PDFLoader } from 'langchain/document_loaders/fs/pdf';
import { DocxLoader } from 'langchain/document_loaders/fs/docx';
import { EPubLoader } from 'langchain/document_loaders/fs/epub';
import { CSVLoader } from 'langchain/document_loaders/fs/csv';
import path from 'path';
import puppeteer, { Page } from 'puppeteer';
import { RecursiveCharacterTextSplitter } from 'langchain/text_splitter';
import { Document } from 'langchain/document';
import getColors from 'get-image-colors';
import sharp from 'sharp';

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
import { GBAdminService } from '../../admin.gbapp/services/GBAdminService.js';
import { GBVMService } from '../../basic.gblib/services/GBVMService.js';
import { DialogKeywords } from '../../basic.gblib/services/DialogKeywords.js';
import { GBMinService } from '../../core.gbapp/services/GBMinService.js';
import { ChatServices } from '../../gpt.gblib/services/ChatServices.js';
import { GBUtil } from '../../../src/util.js';

/**
 * Result for quey on KB data.
 */
export class KBServiceSearchResults {
  public answer: string | GuaribasAnswer;
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
    min: GBMinInstance,
    user,
    step,
    pid,
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

    const instance = min.instance;

    const contentLocale = min.core.getParam<string>(
      min.instance,
      'Default Content Language',
      GBConfigService.get('DEFAULT_CONTENT_LANGUAGE')
    );

    query = await min.conversationalService.translate(min, query, contentLocale);

    GBLogEx.info(min, `Translated query (prompt): ${query}.`);

    // Try simple search first.

    const data = await this.getAnswerByText(instance.instanceId, query.trim());
    if (data) {
      GBLogEx.info(min, `Simple SEARCH called.`);
      return { answer: data.answer, questionId: data.question.questionId };
    }

    if (subjects !== null) {
      const text = KBService.getSubjectItemsSeparatedBySpaces(subjects);
      if (text !== null) {
        query = `${query} ${text}`;
      }
    }
    let returnedScore = 0;
    const key = instance.searchKey ? instance.searchKey : GBServer.globals.minBoot.instance.searchKey;
    const host = instance.searchHost ? instance.searchHost : GBServer.globals.minBoot.instance.searchHost;

    // No direct match found, so Search is used.

    if (key !== null && GBConfigService.get('STORAGE_DIALECT') === 'mssql') {
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

      const client = new SearchClient<any>('https://' + host, 'azuresql-index', {
        key: key
      } as any);

      const results = await client.search(query.substring(0, 499), {
        filter: `instanceId eq ${instance.instanceId} and skipIndex eq false`,
        searchFields: ['content', 'subject1', 'subject2', 'subject3', 'subject4'],
        select: ['instanceId', 'questionId', 'answerId'],
        skip: 0,
        top: 1
      });

      // Searches via Search (Azure Search).

      let found = false;
      for await (const result of results.results) {
        found = true;
        returnedScore = result.score;
        if (returnedScore >= searchScore) {
          const value = await this.getAnswerById(instance.instanceId, result.document.answerId);
          if (value !== null) {
            GBLogEx.info(
              min,
              `SEARCH WILL BE USED with score: ${returnedScore} > required (searchScore): ${searchScore}`
            );

            return { answer: value, questionId: result.document.questionId };
          } else {
            GBLogEx.info(
              min,
              `Index problem. SEARCH WILL NOT be used as answerId ${result.document.answerId} was not found in database,
                returnedScore: ${returnedScore} < required (searchScore): ${searchScore}`
            );

            return { answer: undefined, questionId: 0 };
          }
        }
      }
    }
    GBLogEx.info(
      min,
      `SEARCH returned LOW level score, calling NLP if any,
        returnedScore: ${returnedScore} < required (searchScore): ${searchScore}`
    );

    return await ChatServices.answerByGPT(min, user, query);
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
    GBLogEx.info(min, `Now reading file ${filePath}...`);
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

    GBLogEx.info(min, `Processing ${rows.length} rows from tabular file ${filePath}...`);
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
            GBLogEx.info(min, `[GBImporter] Answer is NULL related to Question '${question}'.`);
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
                GBLogEx.info(min, `[GBImporter] File not found: ${mediaFilename}.`);
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
            const path = DialogKeywords.getGBAIPath(min.botId, `gbdialog`);
            const scriptName = `tmp${GBAdminService.getRndReadableIdentifier()}.docx`;
            const localName = Path.join('work', path, `${scriptName}`);
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

  public async sendAnswer(min: GBMinInstance, channel: string, step: GBDialogStep, answer) {
    answer = typeof answer === 'string' ? answer : answer.content;
    if (answer.endsWith('.mp4')) {
      await this.playVideo(min, min.conversationalService, step, answer, channel);
    } else if (
      answer.endsWith('.ppt') ||
      answer.endsWith('.pptx') ||
      answer.endsWith('.doc') ||
      answer.endsWith('.docx') ||
      answer.endsWith('.xls') ||
      answer.endsWith('.xlsx')
    ) {
      const path = DialogKeywords.getGBAIPath(min.botId, `gbkb`);
      const doc = urlJoin(GBServer.globals.publicAddress, 'kb', path, 'assets', answer);
      const url = `http://view.officeapps.live.com/op/view.aspx?src=${doc}`;
      await this.playUrl(min, min.conversationalService, step, url, channel);
    } else if (answer.endsWith('.pdf')) {
      const path = DialogKeywords.getGBAIPath(min.botId, `gbkb`);
      const url = urlJoin('kb', path, 'assets', answer);
      await this.playUrl(min, min.conversationalService, step, url, channel);
    } else if (answer.format === '.md') {
      await min.conversationalService['playMarkdown'](min, answer, channel, step, GBMinService.userMobile(step));
    } else if (answer.endsWith('.ogg') && process.env.AUDIO_DISABLED !== 'true') {
      await this.playAudio(min, answer, channel, step, min.conversationalService);
    } else {
      await min.conversationalService.sendText(min, step, answer);
    }
  }

  public async addQA(min, questionText, answerText) {
    const pkg = await GuaribasPackage.findOne({
      where: { instanceId: min.instance.instanceId }
    });

    const question = {
      from: 'autodialog',
      to: '',
      subject1: '',
      subject2: '',
      subject3: '',
      subject4: '',
      content: questionText.replace(/["]+/g, ''),
      instanceId: min.instance.instanceId,
      skipIndex: false,
      packageId: pkg.packageId
    };
    const answer = {
      instanceId: min.instance.instanceId,
      content: answerText,
      format: '.txt',
      media: null,
      packageId: pkg.packageId,
      prevId: 0
    };
    const a = await GuaribasAnswer.create(answer);
    question['answerId'] = a.answerId;
    const q = await GuaribasQuestion.create(question);
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

    // Imports menu.xlsx if any.

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
        const path = DialogKeywords.getGBAIPath(instance.botId, `gbkb`);
        const localName = Path.join('work', path, 'articles', file.name);
        let loader = new DocxLoader(localName);
        let doc = await loader.load();

        const answer = {
          instanceId: instance.instanceId,
          content: doc[0].pageContent,
          format: '.md',
          media: file.name,
          packageId: packageId,
          prevId: 0
        };

        data.answers.push(answer);
      } else if (file !== null && file.name.endsWith('.toc.docx')) {
        const path = DialogKeywords.getGBAIPath(instance.botId, `gbkb`);
        const localName = Path.join('work', path, 'articles', file.name);
        const buffer = Fs.readFileSync(localName, { encoding: null });
        var options = {
          buffer: buffer,
          convertImage: async image => {
            const localName = Path.join(
              'work',
              DialogKeywords.getGBAIPath(instance.botId),
              'cache',
              `img-docx${GBAdminService.getRndReadableIdentifier()}.png`
            );
            const url = urlJoin(
              GBServer.globals.publicAddress,
              DialogKeywords.getGBAIPath(instance.botId).replace(/\.[^/.]+$/, ''),
              'cache',
              Path.basename(localName)
            );
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

  async saveHtmlPage(min, url: string, page: Page): Promise<string | null> {
    const response = await page.goto(url);

    if (response.headers && response.status() === 200) {
      const contentType = response.headers()['content-type'];
      if (contentType && contentType.includes('text/html')) {
        const buffer = await page.$eval('*', el => el['innerText']);
        const urlObj = new URL(url);
        const urlPath = urlObj.pathname.endsWith('/') ? urlObj.pathname.slice(0, -1) : urlObj.pathname; // Remove trailing slash if present
        let filename = urlPath.split('/').pop() || 'index'; // Get the filename from the URL path or set it to 'index.html' as default
        filename = `${filename}.html`;
        let path = DialogKeywords.getGBAIPath(min.botId, `gbot`);
        const directoryPath = Path.join(process.env.PWD, 'work', path, 'Website');
        const filePath = Path.join(directoryPath, filename);

        GBLogEx.info(min, `[GBDeployer] Saving Website file in ${filePath}.`);

        Fs.mkdirSync(directoryPath, { recursive: true }); // Create directory recursively if it doesn't exist
        Fs.writeFileSync(filePath, buffer);

        return filePath;
      }
    }
    return null;
  }

  async crawl(
    min,
    url: string,
    visited: Set<string>,
    depth: number,
    maxDepth: number,
    page: Page,
    websiteIgnoreUrls
  ): Promise<string[]> {
    try {
      if (
        depth > maxDepth ||
        visited.has(url) ||
        url.endsWith('.jpg') ||
        url.endsWith('.pdf') ||
        url.endsWith('.jpg') ||
        url.endsWith('.png') ||
        url.endsWith('.mp4')
      ) {
        return [];
      }

      await GBLogEx.info(min, `Processing URL: ${url}.`);

      visited.add(url);

      const filename = await this.saveHtmlPage(min, url, page);

      if (!filename) {
        // If the URL doesn't represent an HTML page, skip crawling its links
        return [];
      }
      const currentDomain = new URL(page.url()).hostname.toLocaleLowerCase();

      let links = await page.evaluate(
        ({ currentDomain, websiteIgnoreUrls }) => {
          const anchors = Array.from(document.querySelectorAll('a')).filter(p => {
            try {
              // Check if urlToCheck contains any of the ignored URLs

              const isIgnored = websiteIgnoreUrls.split(';').some(ignoredUrl => p.href.includes(ignoredUrl));
              console.log(currentDomain);
              console.log(new URL(p.href).hostname);

              return !isIgnored && currentDomain == new URL(p.href).hostname.toLocaleLowerCase();
            } catch (err) {
              return false;
            }
          });

          return anchors.map(anchor => {
            return anchor.href.replace(/#.*/, '');
          });
        },
        { currentDomain, websiteIgnoreUrls }
      );

      if (!Array.isArray(links)) {
        links = [];
      }

      let filteredLinks = [];

      if (links && typeof links[Symbol.iterator] === 'function') {
        filteredLinks = links.filter(l => {
          try {
            new URL(l); // Check if the link is a valid URL
            return !visited.has(l);
          } catch (error) {
            // Ignore invalid URLs
            return false;
          }
        });
      }

      const childLinks = [];
      for (const link of filteredLinks) {
        const links = await this.crawl(min, link, visited, depth + 1, maxDepth, page, websiteIgnoreUrls);
        if (links) {
          childLinks.push(...links);
        }
      }

      return [filename, ...childLinks]; // Include the filename of the cached file
    } catch (error) {
      await GBLogEx.info(min, error);
      return []; // Include the filename of the cached file
    }
  }

  async getLogoByPage(page) {
    const checkPossibilities = async (page, possibilities) => {
      for (const possibility of possibilities) {
        const { tag, attributes } = possibility;

        for (const attribute of attributes) {
          const selector = `${tag}[${attribute}*="logo"]`;
          const elements = await page.$$(selector);

          for (const element of elements) {
            const src = await page.evaluate(el => el.getAttribute('src'), element);
            if (src) {
              return src.split('?')[0];
            }
          }
        }
      }

      return null;
    };

    // Array of possibilities to check for the logo
    const possibilities = [
      { tag: 'img', attributes: ['src', 'alt', 'class'] }, // Check for img elements with specific attributes
      { tag: 'svg', attributes: ['class', 'aria-label'] } // Check for svg elements with specific attributes
      // Add more possibilities as needed
    ];

    return await checkPossibilities(page, possibilities);
  }

  async getFreshPage(browser, url) {
    try {
      if (!browser || browser.isConnected() === false) {
        browser = await puppeteer.launch({ headless: false }); // Change headless to true if you don't want to see the browser window
      }
      const page = await browser.newPage();
      await page.goto(url);
      return page;
    } catch (error) {
      console.error('An error occurred while getting fresh page:', error);
      throw error;
    }
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
    let files = [];

    let website = min.core.getParam<string>(min.instance, 'Website', null);
    const websiteIgnoreUrls = min.core.getParam<string>(min.instance, 'Website Ignore URLs', null);

    if (website) {

      // Removes last slash if any.

      website =website.replace(/\/(?=[^\/]*$)/, ""); 

      Fs.rmSync(min['vectorStorePath'], { recursive: true, force: true });
      let path = DialogKeywords.getGBAIPath(min.botId, `gbot`);
      const directoryPath = Path.join(process.env.PWD, 'work', path, 'Website');
      Fs.rmSync(directoryPath, { recursive: true, force: true });

      let browser = await puppeteer.launch({ headless: false });
      const page = await this.getFreshPage(browser, website);

      let logo = await this.getLogoByPage(page);
      if (logo) {
        path = DialogKeywords.getGBAIPath(min.botId);
        const logoPath = Path.join(process.env.PWD, 'work', path, 'cache');
        const baseUrl = page.url().split('/').slice(0, 3).join('/');
        logo = logo.startsWith('https') ? logo : urlJoin(baseUrl, logo);
        const logoBinary = await page.goto(logo);
        const buffer = await logoBinary.buffer();
        const logoFilename = Path.basename(logo);
        sharp(buffer)
          .resize({
            width: 48,
            height: 48,
            fit: 'inside', // Resize the image to fit within the specified dimensions
            withoutEnlargement: true // Don't enlarge the image if its dimensions are already smaller
          })
          .toFile(Path.join(logoPath, logoFilename));

        await min.core['setConfig'](min, 'Logo', logoFilename);
      }

      // Extract dominant colors from the screenshot

      await page.screenshot({ path: 'screenshot.png' });
      const colors = await getColors('screenshot.png');
      await min.core['setConfig'](min, 'Color1', colors[0].hex());
      await min.core['setConfig'](min, 'Color2', colors[1].hex());

      // Disables images in crawling.

      await page.setRequestInterception(true);
      page.on('request', req => {
        if (req.resourceType() === 'image' || req.resourceType() === 'stylesheet') {
          req.abort();
        } else {
          req.continue();
        }
      });

      page.on('dialog', async dialog => {
        await dialog.dismiss();
      });

      page.setCacheEnabled(false);

      const maxDepth = 1; // Maximum depth of recursion
      const visited = new Set<string>();
      files = files.concat(await this.crawl(min, website, visited, 0, maxDepth, page, websiteIgnoreUrls));

      await browser.close();

      files.shift();

      await CollectionUtil.asyncForEach(files, async file => {
        let content = null;

        const document = await this.loadAndSplitFile(file);
        const flattenedDocuments = document.reduce((acc, val) => acc.concat(val), []);
        const vectorStore = min['vectorStore'];
        await vectorStore.addDocuments(flattenedDocuments);
        await vectorStore.save(min['vectorStorePath']);
      });
    }

    files = await walkPromise(urlJoin(localPath, 'docs'));

    if (!files[0]) {
      GBLogEx.info(min, `[GBDeployer] docs folder not created yet in .gbkb neither a website in .gbot.`);
    } else {
      await CollectionUtil.asyncForEach(files, async file => {
        let content = null;
        let filePath = Path.join(file.root, file.name);

        const document = await this.loadAndSplitFile(filePath);
        const flattenedDocuments = document.reduce((acc, val) => acc.concat(val), []);
        const vectorStore = min['vectorStore'];
        await vectorStore.addDocuments(flattenedDocuments);
        await vectorStore.save(min['vectorStorePath']);
      });
    }
  }

  defaultRecursiveCharacterTextSplitter = new RecursiveCharacterTextSplitter({
    chunkSize: 700,
    chunkOverlap: 50
  });

  markdownRecursiveCharacterTextSplitter = RecursiveCharacterTextSplitter.fromLanguage('markdown', {
    chunkSize: 700,
    chunkOverlap: 50
  });

  private async loadAndSplitFile(filePath: string): Promise<Document<Record<string, unknown>>[]> {
    const fileExtension = path.extname(filePath);
    let loader;
    let documents: Document<Record<string, unknown>>[];
    switch (fileExtension) {
      case '.json':
        loader = new JSONLoader(filePath);
        documents = await loader.loadAndSplit(this.defaultRecursiveCharacterTextSplitter);
        break;
      case '.txt':
        loader = new TextLoader(filePath);
        documents = await loader.loadAndSplit(this.defaultRecursiveCharacterTextSplitter);
        break;
      case '.txt':
        loader = new TextLoader(filePath);
        documents = await loader.loadAndSplit(this.defaultRecursiveCharacterTextSplitter);
        break;
      case '.html':
        loader = new TextLoader(filePath);
        documents = await loader.loadAndSplit(this.defaultRecursiveCharacterTextSplitter);
        break;
      case '.md':
        loader = new TextLoader(filePath);
        documents = await loader.loadAndSplit(this.markdownRecursiveCharacterTextSplitter);
        break;
      case '.pdf':
        loader = new PDFLoader(filePath, { splitPages: false });
        documents = await loader.loadAndSplit(this.defaultRecursiveCharacterTextSplitter);
        break;
      case '.docx':
        loader = new DocxLoader(filePath);
        documents = await loader.loadAndSplit(this.defaultRecursiveCharacterTextSplitter);
        break;
      case '.csv':
        loader = new CSVLoader(filePath);
        documents = await loader.loadAndSplit(this.defaultRecursiveCharacterTextSplitter);
        break;
      case '.epub':
        loader = new EPubLoader(filePath, { splitChapters: false });
        documents = await loader.loadAndSplit(this.defaultRecursiveCharacterTextSplitter);
        break;
      default:
        throw new Error(`Unsupported file extension: ${fileExtension}`);
    }
    return documents;
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
        const description = row._cells[level + 1] ? row._cells[level + 1].text : null;
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

      if (categoryReg) {
        let category = categoryReg[1];

        if (category === 'number') {
          min['nerEngine'].addRegexEntity('number', 'pt', '/d+/gi');
        }
        if (nameReg) {
          let name = nameReg[1];

          min['nerEngine'].addNamedEntityText(category, name, [contentLocale], [name]);
        }
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
    GBLogEx.info(min, `[GBDeployer] Importing: ${localPath}`);

    const p = await deployer.deployPackageToStorage(instance.instanceId, packageName);
    await this.importKbPackage(min, localPath, p, instance);
    GBDeployer.mountGBKBAssets(packageName, min.botId, localPath);
    const service = await AzureDeployerService.createInstance(deployer);
    const searchIndex = instance.searchIndex ? instance.searchIndex : GBServer.globals.minBoot.instance.searchIndex;
    await deployer.rebuildIndex(instance, service.getKBSearchSchema(searchIndex));

    min['groupCache'] = await KBService.getGroupReplies(instance.instanceId);
    await KBService.RefreshNER(min);

    GBLogEx.info(min, `[GBDeployer] Start Bot Server Side Rendering... ${localPath}`);
    const html = await GBSSR.getHTML(min);
    let path = DialogKeywords.getGBAIPath(min.botId, `gbui`);
    path = Path.join(process.env.PWD, 'work', path, 'index.html');
    GBLogEx.info(min, `[GBDeployer] Saving SSR HTML in ${path}.`);
    Fs.writeFileSync(path, html, 'utf8');

    GBLogEx.info(min, `[GBDeployer] Finished import of ${localPath}`);
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
      const path = DialogKeywords.getGBAIPath(min.botId, `gbkb`);
      await conversationalService.sendEvent(min, step, 'play', {
        playerType: 'video',
        data: urlJoin(path, 'videos', answer.content)
      });
    }
  }

  private async undeployPackageFromStorage(instance: any, packageId: number) {
    await GuaribasPackage.destroy({
      where: { instanceId: instance.instanceId, packageId: packageId }
    });
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
