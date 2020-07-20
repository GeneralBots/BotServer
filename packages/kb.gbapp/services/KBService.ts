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

const Path = require('path');
const Fs = require('fs');
const urlJoin = require('url-join');
const marked = require('marked');
const path = require('path');
const asyncPromise = require('async-promises');
const walkPromise = require('walk-promise');
// tslint:disable-next-line:newline-per-chained-call
const { SearchService } = require('azure-search-client');
var Excel = require('exceljs');
import { GBServer } from '../../../src/app';
import { IGBKBService, GBDialogStep, GBLog, IGBConversationalService, IGBCoreService, IGBInstance, GBMinInstance } from 'botlib';
import { Op } from 'sequelize';
import { Sequelize } from 'sequelize-typescript';
import { AzureDeployerService } from '../../azuredeployer.gbapp/services/AzureDeployerService';
import { GuaribasPackage } from '../../core.gbapp/models/GBModel';
import { GBDeployer } from '../../core.gbapp/services/GBDeployer';
import { GuaribasAnswer, GuaribasQuestion, GuaribasSubject } from '../models';
import { Messages } from '../strings';
import { GBConfigService } from './../../core.gbapp/services/GBConfigService';
import { CSService } from '../../customer-satisfaction.gbapp/services/CSService';
import { SecService } from '../../security.gblib/services/SecService';
import { CollectionUtil } from 'pragmatismo-io-framework';

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
    if (subjects === undefined) { return ''; }
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

    return answer != null ? answer.content : null;
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
    return GuaribasAnswer.findOne({
      where: {
        instanceId: instanceId,
        answerId: answerId
      }
    });
  }

  public async getAnswerByText(instanceId: number, text: string): Promise<any> {

    text = text.trim();
    const service = new CSService();
    let question = await service.getQuestionFromAlternateText(instanceId, text);

    if (question !== null) {
      question = await GuaribasQuestion.findOne({
        where: {
          instanceId: instanceId,
          content: { [Op.like]: `%${text}%` }
        }
      });
    }

    if (question !== null) {
      const answer = await GuaribasAnswer.findOne({
        where: {
          instanceId: instanceId,
          answerId: question.answerId
        }
      });

      return Promise.resolve({ question: question, answer: answer });
    }

    return Promise.resolve(undefined);
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

    if (subjects !== null) {
      const text = KBService.getSubjectItemsSeparatedBySpaces(subjects);
      if (text !== null) {
        query = `${query} ${text}`;
      }
    }

    // tslint:disable:no-unsafe-any
    if (instance.searchKey !== null && GBConfigService.get('STORAGE_DIALECT') === 'mssql') {
      const client = new SearchService(instance.searchHost.split('.')[0], instance.searchKey);
      const results = await client.indexes.use(instance.searchIndex)
        .buildQuery()
        .filter((f) => f.eq('instanceId', instance.instanceId))
        .search(query)
        .top(1)
        .executeQuery();

      const values = results.result.value;

      if (values && values.length > 0 && values[0]['@search.score'] >= searchScore) {
        const value = await this.getAnswerById(instance.instanceId, values[0].answerId);
        if (value !== null) {
          return Promise.resolve({ answer: value, questionId: values[0].questionId });
        } else {
          return Promise.resolve({ answer: undefined, questionId: 0 });
        }
      }
    } else {
      const data = await this.getAnswerByText(instance.instanceId, query);
      if (data) {
        return Promise.resolve({ answer: data.answer, questionId: data.question.questionId });
      } else {
        return Promise.resolve({ answer: undefined, questionId: 0 });
      }
    }
  }

  public async getSubjectItems(instanceId: number, parentId: number): Promise<GuaribasSubject[]> {
    const where = { parentSubjectId: parentId, instanceId: instanceId };

    return GuaribasSubject.findAll({
      where: where
    });
  }

  public async getFaqBySubjectArray(from: string, subjects: any): Promise<GuaribasQuestion[]> {
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
        subject4: null
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
        where: { from: from }
      });
    }
  }

  public async importKbTabularFile(
    filePath: string,
    instanceId: number,
    packageId: number
  ): Promise<GuaribasQuestion[]> {

    var workbook = new Excel.Workbook();
    let data = await workbook.xlsx.readFile(filePath);

    let lastQuestionId: number;
    let lastAnswer: GuaribasAnswer;
    let rows = data._worksheets[1]._rows;

    GBLog.info(`Now importing ${rows.length} rows from tabular file ${filePath}...`);

    return asyncPromise.eachSeries(rows, async line => {

      // Skips the first line.

      if (line._cells[0] !== undefined &&
        line._cells[1] !== undefined &&
        line._cells[2] !== undefined &&
        line._cells[3] !== undefined &&
        line._cells[4] !== undefined) {
        // Extracts values from columns in the current line.

        const subjectsText = line._cells[0].text;
        const from = line._cells[1].text;
        const to = line._cells[2].text;
        const question = line._cells[3].text;
        let answer = line._cells[4].text;

        if (!(subjectsText === 'subjects' && from === 'from')
          && (answer !== null && question !== null)) {

          let format = '.txt';

          // Extracts answer from external media if any.

          let media = null;

          if (typeof (answer) !== "string") {
            GBLog.info(`[GBImporter] Answer is NULL related to Question '${question}'.`);
            answer = 'Existe um problema na base de conhecimento. Fui treinado para entender sua pergunta, avise a quem me criou que a resposta não foi informada para esta pergunta.';
          } else if (answer.indexOf('.md') > -1) {
            const mediaFilename = urlJoin(path.dirname(filePath), '..', 'articles', answer);
            if (Fs.existsSync(mediaFilename)) {
              answer = Fs.readFileSync(mediaFilename, 'utf8');
              format = '.md';
              media = path.basename(mediaFilename);
            } else {
              GBLog.info(`[GBImporter] File not found: ${mediaFilename}.`);
              answer = '';
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

          // Now with all the data ready, creates entities in the store.

          const answer1 = await GuaribasAnswer.create({
            instanceId: instanceId,
            content: answer,
            format: format,
            media: media,
            packageId: packageId,
            prevId: lastQuestionId !== null ? lastQuestionId : 0
          });

          const question1 = await GuaribasQuestion.create({
            from: from,
            to: to,
            subject1: subject1,
            subject2: subject2,
            subject3: subject3,
            subject4: subject4,
            content: question,
            instanceId: instanceId,
            answerId: answer1.answerId,
            packageId: packageId
          });

          if (lastAnswer !== undefined && lastQuestionId !== 0) {
            await lastAnswer.update({ nextId: lastQuestionId });
          }
          lastAnswer = answer1;
          lastQuestionId = question1.questionId;

          return Promise.resolve(question1.questionId);
        } else {
          // Skips the header.

          return Promise.resolve(undefined);
        }
      }
    });
  }

  public async sendAnswer(min: GBMinInstance, channel: string, step: GBDialogStep, answer: GuaribasAnswer) {
    if (answer.content.endsWith('.mp4')) {
      await this.playVideo(min, min.conversationalService, step, answer, channel);
    }
    else if (answer.format === '.md') {

      await this.playMarkdown(min, answer, channel, step, min.conversationalService);

    } else if (answer.content.endsWith('.ogg') && process.env.AUDIO_DISABLED !== "true") {

      await this.playAudio(min, answer, channel, step, min.conversationalService);
    } else {
      await min.conversationalService.sendText(min, step, answer.content);
      await min.conversationalService.sendEvent(min, step, 'stop', undefined);
    }
  }

  private async playAudio(min: GBMinInstance, answer: GuaribasAnswer, channel: string, step: GBDialogStep, conversationalService: IGBConversationalService) {
    conversationalService.sendAudio(min, step, answer.content);
  }

  private async playMarkdown(min: GBMinInstance, answer: GuaribasAnswer, channel: string, step: GBDialogStep, conversationalService: IGBConversationalService) {

    let sec = new SecService();
    const member = step.context.activity.from;
    const user = await sec.ensureUser(min.instance.instanceId, member.id,
      member.name, "", "web", member.name);
    const minBoot = GBServer.globals.minBoot as any;

    // Calls language translator.

    let text = await min.conversationalService.translate(min,
      min.instance.translatorKey ? min.instance.translatorKey : minBoot.instance.translatorKey,
      min.instance.translatorEndpoint ? min.instance.translatorEndpoint : minBoot.instance.translatorEndpoint,
      answer.content,
      user.locale ? user.locale : 'pt'
    );

    // Converts from Markdown to HTML.

    marked.setOptions({
      renderer: new marked.Renderer(),
      gfm: true,
      tables: true,
      breaks: false,
      pedantic: false,
      sanitize: false,
      smartLists: true,
      smartypants: false,
      xhtml: false
    });

    // MSFT Translator breaks markdown, so we need to fix it:

    text = text.replace('! [', '![').replace('] (', '](');

    let html = text.replace(`[[embed url=`, process.env.BOT_URL + '/').replace(']]', ''); // TODO: Improve it.

    // According to the channel, formats the output optimized to it.

    if (channel === 'webchat' &&
      GBConfigService.get('DISABLE_WEB') !== 'true') {
      html = marked(text);
      await this.sendMarkdownToWeb(min, step, conversationalService, html, answer);
    }
    else if (channel === 'whatsapp') {

      await conversationalService.sendMarkdownToMobile(min, step, user.userSystemId, text);
    }
    else {
      html = marked(text);
      await min.conversationalService.sendText(min, step, html);
    }
  }

  private async sendMarkdownToWeb(min, step: GBDialogStep, conversationalService: IGBConversationalService, html: string, answer: GuaribasAnswer) {

    let sec = new SecService();
    const member = step.context.activity.from;
    const user = await sec.ensureUser(min.instance.instanceId, member.id,
      member.name, "", "web", member.name);
    const minBoot = GBServer.globals.minBoot as any;
    html = await min.conversationalService.translate(min,
      min.instance.translatorKey ? min.instance.translatorKey : minBoot.instance.translatorKey,
      min.instance.translatorEndpoint ? min.instance.translatorEndpoint : minBoot.instance.translatorEndpoint,
      html,
      user.locale ? user.locale : 'pt'
    );

    const locale = step.context.activity.locale;
    await min.conversationalService.sendText(min, step, Messages[locale].will_answer_projector);
    html = html.replace(/src\=\"kb\//gi, `src=\"../kb/`);
    await conversationalService.sendEvent(min, step, 'play', {
      playerType: 'markdown',
      data: {
        content: html,
        answer: answer,
        prevId: answer.prevId,
        nextId: answer.nextId
      }
    });
  }


  private async playVideo(min, conversationalService: IGBConversationalService,
    step: GBDialogStep, answer: GuaribasAnswer, channel: string) {
    if (channel === "whatsapp") {
      await min.conversationalService.sendFile(min, step, null, answer.content, "");
    } else {
      await conversationalService.sendEvent(min, step, 'play', {
        playerType: 'video',
        data: answer.content
      });
    }
  }

  public async importKbPackage(
    localPath: string,
    packageStorage: GuaribasPackage,
    instance: IGBInstance
  ): Promise<any> {

    // Imports subjects tree into database and return it.

    const subjectFile = urlJoin(localPath, 'subjects.json');

    if (Fs.existsSync(subjectFile)) {
      await this.importSubjectFile(packageStorage.packageId, subjectFile, instance);
    }

    // Import tabular files in the tabular directory.

    await this.importKbTabularDirectory(localPath, instance, packageStorage.packageId);

    // Import remaining .md files in articles directory.

    return await this.importRemainingArticles(localPath, instance, packageStorage.packageId);
  }

  /**
   * Import all .md files in artcles folder that has not been referenced by tabular files.
   */
  public async importRemainingArticles(localPath: string, instance: IGBInstance, packageId: number): Promise<any> {
    const files = await walkPromise(urlJoin(localPath, 'articles'));

    await CollectionUtil.asyncForEach(files, async file => {
      if (file !== null && file.name.endsWith('.md')) {

        let content = await this.getAnswerTextByMediaName(instance.instanceId, file.name);

        if (content === null) {

          const fullFilename = urlJoin(file.root, file.name);
          content = Fs.readFileSync(fullFilename, 'utf-8');

          await GuaribasAnswer.create({
            instanceId: instance.instanceId,
            content: content,
            format: ".md",
            media: file.name,
            packageId: packageId,
            prevId: 0 // TODO: Calculate total rows and increment.
          });
        }
      }
    });
  }
  public async importKbTabularDirectory(localPath: string, instance: IGBInstance, packageId: number): Promise<any> {
    let files = await walkPromise(localPath);

    await CollectionUtil.asyncForEach(files, async file => {
      if (file !== null && file.name.endsWith('.xlsx')) {
        return await this.importKbTabularFile(urlJoin(file.root, file.name), instance.instanceId, packageId);
      }
    })

  }

  public async importSubjectFile(packageId: number, filename: string, instance: IGBInstance): Promise<any> {
    const subjectsLoaded = JSON.parse(Fs.readFileSync(filename, 'utf8'));

    const doIt = async (subjects: GuaribasSubject[], parentSubjectId: number) => {
      return asyncPromise.eachSeries(subjects, async item => {
        const value = await GuaribasSubject.create({
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
          return Promise.resolve(doIt(item.children, value.subjectId));
        } else {
          return Promise.resolve(item);
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

    GBLog.info("Remember to call rebuild index manually after package removal.");

  }

  private async undeployPackageFromStorage(instance: any, packageId: number) {
    await GuaribasPackage.destroy({
      where: { instanceId: instance.instanceId, packageId: packageId }
    });
  }

  /**
   * Deploys a knowledge base to the storage using the .gbkb format.
   *
   * @param localPath Path to the .gbkb folder.
   */
  public async deployKb(core: IGBCoreService, deployer: GBDeployer, localPath: string, min: GBMinInstance) {
    const packageName = Path.basename(localPath);
    GBLog.info(`[GBDeployer] Opening package: ${localPath}`);


    const instance = await core.loadInstanceByBotId(min.botId);
    GBLog.info(`[GBDeployer] Importing: ${localPath}`);
    const p = await deployer.deployPackageToStorage(instance.instanceId, packageName);
    await this.importKbPackage(localPath, p, instance);
    deployer.mountGBKBAssets(packageName,min.botId, localPath);

    await deployer.rebuildIndex(instance, new AzureDeployerService(deployer).getKBSearchSchema(instance.searchIndex));
    GBLog.info(`[GBDeployer] Finished import of ${localPath}`);
  }
}
