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

/**
 * @fileoverview Knowledge base services and logic.
 */
import { SearchClient } from '@azure/search-documents';
import asyncPromise from 'async-promises';
import Excel from 'exceljs';
import fs from 'fs/promises';
import html2md from 'html-to-md';
import { JSONLoader } from 'langchain/document_loaders/fs/json';
import { TextLoader } from 'langchain/document_loaders/fs/text';
import path from 'path';
import getSlug from 'speakingurl';
import urlJoin from 'url-join';
import walkPromise from 'walk-promise';
import { GBServer } from '../../../src/app.js';
import { CSVLoader } from '@langchain/community/document_loaders/fs/csv';
import { DocxLoader } from '@langchain/community/document_loaders/fs/docx';
import { EPubLoader } from '@langchain/community/document_loaders/fs/epub';
import { PDFLoader } from '@langchain/community/document_loaders/fs/pdf';
import svg2img from 'svg2img';
import isICO from 'icojs';
import getColors from 'get-image-colors';
import { Document } from 'langchain/document';
import { RecursiveCharacterTextSplitter } from 'langchain/text_splitter';
import puppeteer, { Page } from 'puppeteer';
import { Jimp } from 'jimp';
import {
  GBDialogStep,
  GBLog,
  GBMinInstance,
  IGBConversationalService,
  IGBCoreService,
  IGBInstance,
  IGBKBService
} from 'botlib';
import mammoth from 'mammoth';
import { parse } from 'node-html-parser';
import pdf from 'pdf-extraction';
import { CollectionUtil } from 'pragmatismo-io-framework';
import { Op } from 'sequelize';
import { Sequelize } from 'sequelize-typescript';
import textract from 'textract';
import { GBUtil } from '../../../src/util.js';
import { GBAdminService } from '../../admin.gbapp/services/GBAdminService.js';
import { AzureDeployerService } from '../../azuredeployer.gbapp/services/AzureDeployerService.js';
import webp from 'webp-converter';
import os from 'os';
import { DialogKeywords } from '../../basic.gblib/services/DialogKeywords.js';
import { GBVMService } from '../../basic.gblib/services/GBVMService.js';
import { GuaribasPackage } from '../../core.gbapp/models/GBModel.js';
import { GBDeployer } from '../../core.gbapp/services/GBDeployer.js';
import { GBLogEx } from '../../core.gbapp/services/GBLogEx.js';
import { GBMinService } from '../../core.gbapp/services/GBMinService.js';
import { GBSSR } from '../../core.gbapp/services/GBSSR.js';
import { CSService } from '../../customer-satisfaction.gbapp/services/CSService.js';
import { ChatServices } from '../../llm.gblib/services/ChatServices.js';
import { GuaribasAnswer, GuaribasQuestion, GuaribasSubject } from '../models/index.js';
import { GBConfigService } from './../../core.gbapp/services/GBConfigService.js';

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

    let packagePath = /(http[s]?:\/\/)?([^\/\s]+\/)(.*)/gi;
    const botId = url.replace(packagePath, ($0, $1, $2, $3) => {
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

    return await ChatServices.answerByLLM(step.context.activity['pid'], min, user, query);
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
    GBLogEx.info(min, `Now reading file ${path.basename(filePath)}...`);
    const workbook = new Excel.Workbook();

    let data;
    if (filePath.endsWith('.xlsx')) {
      data = await workbook.xlsx.readFile(filePath);
    } else if (filePath.endsWith('.csv')) {
      data = await workbook.csv.readFile(filePath);
    }

    let lastQuestionId: number;
    let lastAnswer: GuaribasAnswer;

    // Finds a valid worksheet because Excel returns empty slots
    // when loading worksheets collection.

    let worksheet = data;
    if (!worksheet._rows) {
      for (let t = 0; t < data.worksheets.length; t++) {
        worksheet = data.worksheets[t];
        if (worksheet) {
          break;
        }
      }
    }

    const rows = worksheet._rows;
    const answers = [];
    const questions = [];

    GBLogEx.info(min, `Processing ${rows?.length} rows from ${path.basename(filePath)}...`);
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
            if (await GBUtil.exists(mediaFilename)) {
              // Tries to load .docx file from Articles folder.

              if (answer.indexOf('.docx') > -1) {
                answer = await this.getTextFromFile(filePath);
              } else {
                // Loads normally markdown file.

                answer = await fs.readFile(mediaFilename, 'utf8');
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
            const packagePath = GBUtil.getGBAIPath(min.botId, `gbdialog`);
            const scriptName = `tmp${GBAdminService.getRndReadableIdentifier()}.docx`;
            const localName = path.join('work', packagePath, `${scriptName}`);
            fs.writeFile(localName, code, { encoding: null });
            answer = scriptName;

            const vm = new GBVMService();
            await vm.loadDialog(path.basename(localName), path.dirname(localName), min);
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
      const packagePath = GBUtil.getGBAIPath(min.botId, `gbkb`);
      const doc = urlJoin(GBServer.globals.publicAddress, 'kb', packagePath, 'assets', answer);
      const url = `http://view.officeapps.live.com/op/view.aspx?src=${doc}`;
      await this.playUrl(min, min.conversationalService, step, url, channel);
    } else if (answer.endsWith('.pdf')) {
      const packagePath = GBUtil.getGBAIPath(min.botId, `gbkb`);
      const url = urlJoin('kb', packagePath, 'assets', answer);
      await this.playUrl(min, min.conversationalService, step, url, channel);
    } else if (answer.format === '.md') {
      await min.conversationalService['playMarkdown'](min, answer, channel, step, GBMinService.userMobile(step));
    } else if (answer.endsWith('.ogg') && process.env.AUDIO_DISABLED !== 'true') {
      await this.playAudio(min, answer, channel, step, min.conversationalService);
    } else if (answer.startsWith('![')) {

      // Checks for text after the image markdown, after the element 4, there are text blocks.

      const removeMarkdownImages = (text: string) => {
        return text.replace(/!\[[^\]]*\](?:\([^)]*\)|\[[^\]]*\])/g, '').trim();
      }

      if (removeMarkdownImages(answer)) {
        await min.conversationalService.sendText(min, step, answer);
      }
      else {
        const urlMatch = answer.match(/!?\[.*?\]\((.*?)\)/);
        const url = urlMatch ? urlMatch[1] : null;
        await this.showImage(min, min.conversationalService, step, url, channel)
      }

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

    if ((await GBUtil.exists(subjectFile)) || (await GBUtil.exists(menuFile))) {
      await this.importSubjectFile(packageStorage.packageId, subjectFile, menuFile, instance);
    }

    // Import tabular files in the tabular directory.

    await this.importKbTabularDirectory(localPath, min, packageStorage.packageId);

    // Import remaining .md files in articles directory.

    await this.importRemainingArticles(min, localPath, instance, packageStorage.packageId);

    // Import docs files in .docx directory.

    return await this.importDocs(min, localPath, instance, packageStorage.packageId);
  }

  /**
   * Import all .md files in articles folder that has not been referenced by tabular files.
   */
  public async importRemainingArticles(
    min: GBMinInstance,
    localPath: string,
    instance: IGBInstance,
    packageId: number
  ): Promise<any> {
    const files = await walkPromise(urlJoin(localPath, 'articles'));
    const data = { questions: [], answers: [] };

    await CollectionUtil.asyncForEach(files, async file => {
      if (file !== null && file.name.endsWith('.md')) {
        let content = await this.getAnswerTextByMediaName(instance.instanceId, file.name);

        if (content === null) {
          const fullFilename = urlJoin(file.root, file.name);
          content = await fs.readFile(fullFilename, 'utf-8');

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
        let packagePath = GBUtil.getGBAIPath(instance.botId, `gbkb`);
        const localName = path.join('work', packagePath, 'articles', file.name);
        let loader = new DocxLoader(localName);
        let doc = await loader.load();
        let content = doc[0].pageContent;

        if (file.name.endsWith('zap.docx')) {
          await min.whatsAppDirectLine.createOrUpdateTemplate(min, file.name, content);
        }

        const answer = {
          instanceId: instance.instanceId,
          content: content,
          format: '.md',
          media: file.name,
          packageId: packageId,
          prevId: 0
        };

        data.answers.push(answer);
      } else if (file !== null && file.name.endsWith('.toc.docx')) {
        const packagePath = GBUtil.getGBAIPath(instance.botId, `gbkb`);
        const localName = path.join('work', packagePath, 'articles', file.name);
        const buffer = await fs.readFile(localName, { encoding: null });
        var options = {
          buffer: buffer,
          convertImage: async image => {
            const localName = path.join(
              'work',
              GBUtil.getGBAIPath(instance.botId),
              'cache',
              `img-docx${GBAdminService.getRndReadableIdentifier()}.png`
            );
            const url = urlJoin(
              GBServer.globals.publicAddress,
              GBUtil.getGBAIPath(instance.botId).replace(/\.[^/.]+$/, ''),
              'cache',
              path.basename(localName)
            );
            const buffer = await image.read();
            await fs.writeFile(localName, buffer, { encoding: null });
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

  async crawl(
    min,
    url: string,
    visited: Set<string>,
    depth: number,
    maxDepth: number,
    page: Page,
    websiteIgnoreUrls, maxDocuments: number
  ): Promise<string[]> {
    try {
      if (
        (maxDocuments < visited.size) ||
        (depth > maxDepth && !url.endsWith('pdf')) ||
        visited.has(url) ||
        url.endsWith('.jpg') ||
        url.endsWith('.png') ||
        url.endsWith('.mp4')
      ) {
        return [];
      }

      await GBLogEx.info(min, `Crawling: ${url}.`);
      visited.add(url);

      const packagePath = GBUtil.getGBAIPath(min.botId, `gbot`);
      const directoryPath = path.join(process.env.PWD, 'work', packagePath, 'Website');
      const filename = await KBService.savePage(min, url, page, directoryPath);

      if (!filename) {
        // If the URL doesn't represent an HTML/PDF page, skip crawling its links
        return [];
      }
      const currentDomain = new URL(page.url()).hostname;

      let links = await page.evaluate(
        ({ currentDomain, websiteIgnoreUrls }) => {
          const anchors = Array.from(document.querySelectorAll('a')).filter(p => {
            try {
              // Check if urlToCheck contains any of the ignored URLs

              var isIgnored = false;
              if (websiteIgnoreUrls) {
                websiteIgnoreUrls.split(';').some(ignoredUrl => p.href.includes(ignoredUrl));
              }

              return !isIgnored && currentDomain == new URL(p.href).hostname;
            } catch (error) {
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
        const links = await this.crawl(min, link, visited, depth + 1, maxDepth, page, websiteIgnoreUrls, maxDocuments);
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

  async getLogoByPage(min, page) {
    const checkPossibilities = async (page, possibilities) => {
      try {
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
      } catch (error) {
        await GBLogEx.info(min, error);
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
    const maxDepth = min.core.getParam<number>(min.instance, 'Website Depth', 1);
    const MAX_DOCUMENTS = 50;
    const maxDocuments = min.core.getParam<number>(min.instance, 'Website Max Documents', MAX_DOCUMENTS);
    const websiteIgnoreUrls = min.core.getParam<[]>(min.instance, 'Website Ignore URLs', null);
    GBLogEx.info(min, `Website: ${website}, Max Depth: ${maxDepth}, Website Max Documents: ${maxDocuments}, Ignore URLs: ${websiteIgnoreUrls}`);

    let shouldSave = false;

    if (website) {
      // Removes last slash if any.

      website.endsWith('/') ? website.substring(0, website.length - 1) : website;

      let packagePath = GBUtil.getGBAIPath(min.botId, `gbot`);
      const directoryPath = path.join(process.env.PWD, 'work', packagePath, 'Website');
      fs.rm(directoryPath, { recursive: true, force: true });

      let browser = await puppeteer.launch({ headless: false });
      const page = await this.getFreshPage(browser, website);

      let logo = await this.getLogoByPage(min, page);
      if (logo) {
        packagePath = GBUtil.getGBAIPath(min.botId);

        const baseUrl = page.url().split('/').slice(0, 3).join('/');
        logo = logo.startsWith('https') ? logo : urlJoin(baseUrl, logo);

        const logoBinary = await page.goto(logo);
        let buffer = await logoBinary.buffer();
        let logoFilename = 'extracted-logo.png';

        // Replace sharp with jimp
        if (buffer.slice(0, 4).toString('hex') === '00000100') {
          // Convert ICO to PNG
          const images = await isICO.parseICO(buffer, 'image/x-icon');
          if (!images || images.length === 0) {
            throw new Error('Failed to parse ICO file');
          }
          buffer = Buffer.from(images[0].buffer);
        } else if (buffer.slice(0, 4).toString('hex') === '52494646' &&
          buffer.slice(8, 12).toString('hex') === '57454250') {

          // Convert WebP to PNG using temporary files
          const tempWebP = path.join(os.tmpdir(), `temp-${Date.now()}.webp`);
          const tempPNG = path.join(os.tmpdir(), `temp-${Date.now()}.png`);

          await fs.writeFile(tempWebP, buffer);
          await webp.dwebp(tempWebP, tempPNG, "-o");

          buffer = await fs.readFile(tempPNG);

        } else if (buffer.toString().includes('<svg')) {

          // For SVG files, convert using svg2img

          buffer = await new Promise((resolve, reject) => {
            svg2img(buffer, {
              resvg: {
                fitTo: {
                  mode: 'width', // or height
                  value: 48,
                },
              }
            }, (error: any, buffer: Buffer) => {
              if (error) {
                reject(error);
              } else {
                resolve(buffer);
              }
            });
          });
        }

        // Replace sharp with jimp
        const image = await Jimp.read(buffer);
        await image.scaleToFit({ w: 48, h: 48 });
        packagePath = path.join(process.env.PWD, 'work', packagePath);

        const logoPath = path.join(packagePath, 'cache', logoFilename);
        await (image as any).write(logoPath);
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

      const visited = new Set<string>();
      files = files.concat(await this.crawl(min, website, visited, 0, maxDepth, page, websiteIgnoreUrls, maxDocuments));

      await browser.close();

      files.shift();

      GBLogEx.info(min, `Vectorizing ${files.length} file(s)...`);

      if (await GBUtil.exists(min['vectorStorePath'])) {

        GBLogEx.info(min, `Cleaning vector store: ${min['vectorStorePath']}...`)
        const gbkbPath = GBUtil.getGBAIPath(min.botId, 'gbkb');
        min['vectorStorePath'] = path.join('work', gbkbPath, 'docs-vectorized');
        min['vectorStore'] = await min.deployService['loadOrCreateEmptyVectorStore'](min);

      }

      await CollectionUtil.asyncForEach(files, async file => {
        let content = null;
        shouldSave = true;

        try {
          const document = await this.loadAndSplitFile(file);
          const flattenedDocuments = document.reduce((acc, val) => acc.concat(val), []);
          await min['vectorStore'].addDocuments(flattenedDocuments);
        } catch (error) {
          GBLogEx.info(min, `Ignore processing of ${file}. ${GBUtil.toYAML(error)}`);
        }
      });


    }

    files = await walkPromise(urlJoin(localPath, 'docs'));

    if (files[0]) {
      shouldSave = true;
      GBLogEx.info(min, `Add embeddings from .gbkb: ${files.length} files being processed...`);
      await CollectionUtil.asyncForEach(files, async file => {
        let content = null;
        let filePath = path.join(file.root, file.name);

        const document = await this.loadAndSplitFile(filePath);
        const flattenedDocuments = document.reduce((acc, val) => acc.concat(val), []);
        await min['vectorStore'].addDocuments(flattenedDocuments);
      });
    }
    if (shouldSave && min['vectorStore']) {
      await min['vectorStore'].save(min['vectorStorePath']);
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
      if (file !== null && (file.name.endsWith('.xlsx') || file.name.endsWith('.csv'))) {
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
    if (await GBUtil.exists(menuFile)) {
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
      subjectsLoaded = JSON.parse(await fs.readFile(filename, 'utf8'));
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
    const packageName = path.basename(localPath);
    const instance = await core.loadInstanceByBotId(min.botId);
    GBLogEx.info(min, `Publishing: ${path.basename(localPath)}...`);

    const p = await deployer.deployPackageToStorage(instance.instanceId, packageName);
    await this.importKbPackage(min, localPath, p, instance);
    GBDeployer.mountGBKBAssets(packageName, min.botId, localPath);

    if (GBConfigService.get('STORAGE_NAME')) {
      const service = await AzureDeployerService.createInstance(deployer);
      const searchIndex = instance.searchIndex ? instance.searchIndex : GBServer.globals.minBoot.instance.searchIndex;
      await deployer.rebuildIndex(instance, service.getKBSearchSchema(searchIndex));
    }
    min['groupCache'] = await KBService.getGroupReplies(instance.instanceId);
    await KBService.RefreshNER(min);

    const ssr = min.core.getParam<boolean>(min.instance, 'SSR', false);

    if (ssr) {
      GBLogEx.info(min, `Start Bot Server Side Rendering... ${localPath}`);
      const html = await GBSSR.getHTML(min);
      let packagePath = GBUtil.getGBAIPath(min.botId, `gbui`);
      packagePath = path.join(process.env.PWD, 'work', packagePath, 'index.html');
      GBLogEx.info(min, `Saving SSR HTML in ${packagePath}.`);
      await fs.writeFile(packagePath, html, 'utf8');
    }

    GBLogEx.info(min, `Done publishing of: ${path.basename(localPath)}.`);
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
  public async showImage(
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
        playerType: 'image',
        data: url
      });
    }
  }
  public async playUrl(
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
      const packagePath = GBUtil.getGBAIPath(min.botId, `gbkb`);
      await conversationalService.sendEvent(min, step, 'play', {
        playerType: 'video',
        data: urlJoin(packagePath, 'videos', answer.content)
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

  public static async savePage(
    min: GBMinInstance,
    url: string,
    page: Page,
    directoryPath: string
  ): Promise<string | null> {
    try {
      // Check if the directory exists, create it if not.

      const directoryExists = await GBUtil.exists(directoryPath);
      if (!directoryExists) {
        await fs.mkdir(directoryPath, { recursive: true }); // Create directory if it doesn't exist
      }

      // Check if the URL is for a downloadable file (e.g., .pdf).

      if (
        url.endsWith('.pdf') ||
        url.endsWith('.docx') ||
        url.endsWith('.csv') ||
        url.endsWith('.epub') ||
        url.endsWith('.xml') ||
        url.endsWith('.json') ||
        url.endsWith('.txt')
      ) {
        const response = await fetch(url);

        if (!response.ok) {
          throw new Error('Failed to download the file');
        }

        const buffer = await response.arrayBuffer(); // Convert response to array buffer
        const fileName = path.basename(url); // Extract file name from URL
        const filePath = path.join(directoryPath, fileName); // Create file path

        const data = new Uint8Array(buffer);
        await fs.writeFile(filePath, data);

        return filePath; // Return the saved file path
      } else {
        await page.goto(url, {
          waitUntil: 'domcontentloaded', // Changed to only wait for DOM
          timeout: 10000 // Reduced timeout to 10 seconds
        });

        // Stop all scripts and requests
        await page.setRequestInterception(true);
        page.on('request', request => request.abort());


        const parsedUrl = new URL(url);

        // Get the last part of the URL path or default to 'index' if empty
        const pathParts = parsedUrl.pathname.split('/').filter(Boolean); // Remove empty parts
        const lastPath = pathParts.length > 0 ? pathParts[pathParts.length - 1] : 'index';
        const flatLastPath = lastPath.replace(/\W+/g, '-'); // Flatten the last part of the path

        const fileName = `${flatLastPath}.html`;
        const filePath = path.join(directoryPath, fileName);

        const htmlContent = await page.content();

        // Convert HTML to Markdown using html2md
        const markdownContent = html2md(htmlContent);

        // Write Markdown content to file
        await fs.writeFile(filePath, markdownContent);

        return filePath;
      }
    } catch (error) {
      GBLogEx.info(min, `Cannot save: ${url}. ${GBUtil.toYAML(error)}`);
      return null;
    }
  }
}
