/*****************************************************************************\
|                                               ( )_  _                       |
|    _ _    _ __   _ _    __    ___ ___     _ _ | ,_)(_)  ___   ___     _     |
|   ( '_`\ ( '__)/'_` ) /'_ `\/' _ ` _ `\ /'_` )| |  | |/',__)/' _ `\ /'_`\   |
|   | (_) )| |  ( (_| |( (_) || ( ) ( ) |( (_| || |_ | |\__, \| ( ) |( (_) )  |
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

const logger = require("../../../src/logger")
const Path = require("path")
const Fs = require("fs")
const promise = require('bluebird')
const parse = promise.promisify(require('csv-parse'))
const UrlJoin = require("url-join")
const marked = require("marked")
const path = require("path")
const asyncPromise = require('async-promises')
const walkPromise = require('walk-promise')
import { Messages } from "../strings";

import { Sequelize } from 'sequelize-typescript'
import { GBConfigService } from './../../core.gbapp/services/GBConfigService'
import { GuaribasQuestion, GuaribasAnswer, GuaribasSubject } from "../models"
import { IGBCoreService, IGBConversationalService, IGBInstance } from "botlib"
import { AzureSearch } from "pragmatismo-io-framework"
import { GBDeployer } from "../../core.gbapp/services/GBDeployer"
import { GuaribasPackage } from "../../core.gbapp/models/GBModel"

export class KBServiceSearchResults {
  answer: GuaribasAnswer
  questionId: number
}

export class KBService {

  sequelize: Sequelize

  constructor(sequelize: Sequelize) {
    this.sequelize = sequelize
  }

  async getQuestionById(
    instanceId: number,
    questionId: number
  ): Promise<GuaribasQuestion> {
    return GuaribasQuestion.findOne({
      where: {
        instanceId: instanceId,
        questionId: questionId
      }
    })
  }

  async getAnswerById(
    instanceId: number,
    answerId: number
  ): Promise<GuaribasAnswer> {
    return GuaribasAnswer.findOne({
      where: {
        instanceId: instanceId,
        answerId: answerId
      }
    })
  }

  async getAnswerByText(
    instanceId: number,
    text: string
  ): Promise<any> {

    const Op = Sequelize.Op

    let question = await GuaribasQuestion.findOne({
      where: {
        instanceId: instanceId,
        content: { [Op.like]: `%${text.trim()}%` }
      }
    })

    if (question) {
      let answer = await GuaribasAnswer.findOne({
        where: {
          instanceId: instanceId,
          answerId: question.answerId
        }
      })
      return Promise.resolve({ question: question, answer: answer })
    }
    return Promise.resolve(null)
  }

  async addAnswer(obj: GuaribasAnswer): Promise<GuaribasAnswer> {
    return new Promise<GuaribasAnswer>(
      (resolve, reject) => {
        GuaribasAnswer.create(obj).then(item => {
          resolve(item)
        }).error((reason) => {
          reject(reason)
        })
      })
  }

  async ask(
    instance: IGBInstance,
    query: string,
    searchScore: number,
    subjects: GuaribasSubject[]
  ): Promise<KBServiceSearchResults> {

    // Builds search query.

    query = query.toLowerCase()
    query = query.replace("?", " ")
    query = query.replace("!", " ")
    query = query.replace(".", " ")
    query = query.replace("/", " ")
    query = query.replace("\\", " ")

    if (subjects) {
      let text = KBService.getSubjectItemsSeparatedBySpaces(subjects)
      if (text) {
        query = `${query} ${text}`
      }
    }
    // TODO: Filter by instance. what = `${what}&$filter=instanceId eq ${instanceId}`
    try {
      if (instance.searchKey && GBConfigService.get("STORAGE_DIALECT") == "mssql") {
        let service = new AzureSearch(
          instance.searchKey,
          instance.searchHost,
          instance.searchIndex,
          instance.searchIndexer
        )
        let results = await service.search(query)
        if (results && results.length > 0 &&
          results[0]["@search.score"] >= searchScore) {
          let value = await this.getAnswerById(
            instance.instanceId,
            results[0].answerId)
          if (value) {
            return Promise.resolve({ answer: value, questionId: results[0].questionId })
          }
          else {
            return Promise.resolve({ answer: null, questionId: 0 })
          }
        }
      } else {
        let data = await this.getAnswerByText(instance.instanceId, query)
        if (data) {
          return Promise.resolve(
            { answer: data.answer, questionId: data.question.questionId })
        } else {
          return Promise.resolve({ answer: null, questionId: 0 })
        }
      }
    }
    catch (reason) {
      return Promise.reject(new Error(reason));
    }
  }

  getSearchSchema(indexName) {
    return {
      name: indexName,
      fields: [
        {
          name: "questionId",
          type: "Edm.String",
          searchable: false,
          filterable: false,
          retrievable: true,
          sortable: false,
          facetable: false,
          key: true
        },
        {
          name: "subject1",
          type: "Edm.String",
          searchable: true,
          filterable: false,
          retrievable: false,
          sortable: false,
          facetable: false,
          key: false
        },
        {
          name: "subject2",
          type: "Edm.String",
          searchable: true,
          filterable: false,
          retrievable: false,
          sortable: false,
          facetable: false,
          key: false
        },
        {
          name: "subject3",
          type: "Edm.String",
          searchable: true,
          filterable: false,
          retrievable: false,
          sortable: false,
          facetable: false,
          key: false
        },
        {
          name: "subject4",
          type: "Edm.String",
          searchable: true,
          filterable: false,
          retrievable: false,
          sortable: false,
          facetable: false,
          key: false
        },
        {
          name: "content",
          type: "Edm.String",
          searchable: true,
          filterable: false,
          retrievable: false,
          sortable: false,
          facetable: false,
          key: false
        },
        {
          name: "answerId",
          type: "Edm.Int32",
          searchable: false,
          filterable: false,
          retrievable: true,
          sortable: false,
          facetable: false,
          key: false
        },
        {
          name: "instanceId",
          type: "Edm.Int32",
          searchable: false,
          filterable: true,
          retrievable: true,
          sortable: false,
          facetable: false,
          key: false
        },
        {
          name: "packageId",
          type: "Edm.Int32",
          searchable: false,
          filterable: true,
          retrievable: true,
          sortable: false,
          facetable: false,
          key: false
        }
      ],
      scoringProfiles: [],
      defaultScoringProfile: null,
      corsOptions: null
    }
  }

  static getFormattedSubjectItems(subjects: GuaribasSubject[]) {
    if (!subjects) return ""
    let out = []
    subjects.forEach(subject => {
      out.push(subject.title)
    })
    return out.join(", ")
  }

  static getSubjectItemsSeparatedBySpaces(subjects: GuaribasSubject[]) {
    let out = []
    subjects.forEach(subject => {
      out.push(subject.internalId)
    })
    return out.join(" ")
  }

  async getSubjectItems(
    instanceId: number,
    parentId: number
  ): Promise<GuaribasSubject[]> {
    var where = { parentSubjectId: parentId, instanceId: instanceId }
    return GuaribasSubject.findAll({
      where: where
    })
  }

  async getFaqBySubjectArray(from: string, subjects: any): Promise<GuaribasQuestion[]> {
    let where = {
      from: from
    }

    if (subjects) {
      if (subjects[0]) {
        where["subject1"] = subjects[0].internalId
      }

      if (subjects[1]) {
        where["subject2"] = subjects[1].internalId
      }

      if (subjects[2]) {
        where["subject3"] = subjects[2].internalId
      }

      if (subjects[3]) {
        where["subject4"] = subjects[3].internalId
      }
    }
    return await GuaribasQuestion.findAll({
      where: where
    })
  }

  async importKbTabularFile(
    filePath: string,
    instanceId: number,
    packageId: number
  ): Promise<GuaribasQuestion[]> {

    let file = Fs.readFileSync(filePath, "UCS-2")
    let opts = {
      delimiter: "\t"
    }

    let lastQuestion: GuaribasQuestion;
    let lastAnswer: GuaribasAnswer;

    let data = await parse(file, opts)
    return asyncPromise.eachSeries(data, async line => {

      // Extracts values from columns in the current line.

      let subjectsText = line[0]
      var from = line[1]
      var to = line[2]
      var question = line[3]
      var answer = line[4]

      // Skips the first line.

      if (!(subjectsText === "subjects" && from == "from")) {
        let format = ".txt"

        // Extracts answer from external media if any.

        if (answer.indexOf(".md") > -1) {
          let mediaFilename = UrlJoin(path.dirname(filePath), "..", "articles", answer)
          if (Fs.existsSync(mediaFilename)) {
            answer = Fs.readFileSync(mediaFilename, "utf8")
            format = ".md"
          } else {
            logger.info(`[GBImporter] File not found: ${mediaFilename}.`)
            answer = ""
          }
        }

        // Processes subjects hierarchy splitting by dots.

        let subjectArray = subjectsText.split(".")
        let subject1: string, subject2: string, subject3: string,
          subject4: string
        var indexer = 0

        subjectArray.forEach(element => {
          if (indexer == 0) {
            subject1 = subjectArray[indexer].substring(0, 63)
          } else if (indexer == 1) {
            subject2 = subjectArray[indexer].substring(0, 63)
          } else if (indexer == 2) {
            subject3 = subjectArray[indexer].substring(0, 63)
          } else if (indexer == 3) {
            subject4 = subjectArray[indexer].substring(0, 63)
          }
          indexer++
        })

        // Now with all the data ready, creates entities in the store.

        let answer1 = await GuaribasAnswer.create({
          instanceId: instanceId,
          content: answer,
          format: format,
          packageId: packageId,
          prevId: lastQuestion ? lastQuestion.questionId : 0,
        })

        let question1 = await GuaribasQuestion.create({
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
        })

        if (lastAnswer && lastQuestion) {
          await lastAnswer.updateAttributes({ nextId: lastQuestion.questionId })
        }
        lastAnswer = answer1
        lastQuestion = question1

        return Promise.resolve(lastQuestion)

      } else {

        // Skips the header.

        return Promise.resolve(null)
      }
    })
  }

  async sendAnswer(conversationalService: IGBConversationalService,
    dc: any, answer: GuaribasAnswer) {

    if (answer.content.endsWith('.mp4')) {
      await conversationalService.sendEvent(dc, "play", {
        playerType: "video",
        data: answer.content
      })
    } else if (answer.content.length > 140 &&
      dc.context._activity.channelId === "webchat") {
      const locale = dc.context.activity.locale;

      await dc.context.sendActivity(Messages[locale].will_answer_projector) // TODO: Handle rnd.
      var html = answer.content

      if (answer.format === ".md") {
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
        })
        html = marked(answer.content)
      }
      await conversationalService.sendEvent(dc, "play",
        {
          playerType: "markdown", data: {
            content: html, answer: answer,
            prevId: answer.prevId, nextId: answer.nextId
          }
        })
    } else {
      await dc.context.sendActivity(answer.content)
      await conversationalService.sendEvent(dc, "stop", null)
    }
  }

  async importKbPackage(
    localPath: string,
    packageStorage: GuaribasPackage,
    instance: IGBInstance
  ): Promise<any> {

    // Imports subjects tree into database and return it.

    await this.importSubjectFile(
      packageStorage.packageId,
      UrlJoin(localPath, "subjects.json"),
      instance)

    // Import all .tsv files in the tabular directory.

    return this.importKbTabularDirectory(
      localPath,
      instance,
      packageStorage.packageId
    )
  }


  async importKbTabularDirectory(
    localPath: string,
    instance: IGBInstance,
    packageId: number
  ): Promise<any> {

    let files = await walkPromise(UrlJoin(localPath, "tabular"))

    return Promise.all(files.map(async file => {
      if (file.name.endsWith(".tsv")) {
        return this.importKbTabularFile(
          UrlJoin(file.root, file.name),
          instance.instanceId,
          packageId)
      }
    }))

  }

  async importSubjectFile(
    packageId: number,
    filename: string,
    instance: IGBInstance
  ): Promise<any> {
    var subjects = JSON.parse(Fs.readFileSync(filename, "utf8"))

    const doIt = async (subjects: GuaribasSubject[], parentSubjectId: number) => {
      return asyncPromise.eachSeries(subjects, async item => {
        let mediaFilename = item.id + ".png"

        let value = await GuaribasSubject.create({
          internalId: item.id,
          parentSubjectId: parentSubjectId,
          instanceId: instance.instanceId,
          from: item.from,
          to: item.to,
          title: item.title,
          description: item.description,
          packageId: packageId
        })

        if (item.children) {
          return Promise.resolve(doIt(item.children, value.subjectId))
        }
        else {
          return Promise.resolve(item)
        }
      })
    }
    return doIt(subjects.children, null)
  }

  async undeployKbFromStorage(
    instance: IGBInstance,
    deployer: GBDeployer,
    packageId: number
  ) {

    await GuaribasQuestion.destroy({
      where: { instanceId: instance.instanceId, packageId: packageId }
    })
    await GuaribasAnswer.destroy({
      where: { instanceId: instance.instanceId, packageId: packageId }
    })
    await GuaribasSubject.destroy({
      where: { instanceId: instance.instanceId, packageId: packageId }
    })
    await GuaribasPackage.destroy({
      where: { instanceId: instance.instanceId, packageId: packageId }
    })

    await deployer.rebuildIndex(instance)
  }

  /**
  * Deploys a knowledge base to the storage using the .gbkb format.
  * 
  * @param localPath Path to the .gbkb folder.
  */
  async deployKb(core: IGBCoreService, deployer: GBDeployer, localPath: string) {
    let packageType = Path.extname(localPath)
    let packageName = Path.basename(localPath)
    logger.info(`[GBDeployer] Opening package: ${localPath}`)
    let packageObject = JSON.parse(
      Fs.readFileSync(UrlJoin(localPath, "package.json"), "utf8")
    )

    let instance = await core.loadInstance(packageObject.botId)
    logger.info(`[GBDeployer] Importing: ${localPath}`)
    let p = await deployer.deployPackageToStorage(
      instance.instanceId,
      packageName)
    await this.importKbPackage(localPath, p, instance)

    deployer.rebuildIndex(instance)
    logger.info(`[GBDeployer] Finished import of ${localPath}`)
  }
}
