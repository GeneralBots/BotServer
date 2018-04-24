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
| but WITHOUT ANY WARRANTY; without even the implied warranty of              |
| MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the                |
| GNU Affero General Public License for more details.                         |
|                                                                             |
| "General Bots" is a registered trademark of Pragmatismo.io.                 |
| The licensing of the program under the AGPLv3 does not imply a              |
| trademark license. Therefore any rights, title and interest in              |
| our trademarks remain entirely with us.                                     |
|                                                                             |
\*****************************************************************************/

const logger = require("../../../src/logger");
const Path = require("path");
const Fs = require("fs");
const Parse = require("csv-parse");
const Async = require("async");
const UrlJoin = require("url-join");
const Walk = require("fs-walk");
const WaitUntil = require("wait-until");
const marked = require("marked");

import { GuaribasQuestion, GuaribasAnswer, GuaribasSubject } from "../models";
import { GBServiceCallback, IGBCoreService, IGBConversationalService, IGBInstance } from "botlib";
import { AzureSearch } from "pragmatismo-io-framework";
import { GBCoreService } from 'deploy/core.gbapp/services/GBCoreService';
import { GBDeployer } from "../../core.gbapp/services/GBDeployer";
import { GBConversationalService } from "../../core.gbapp/services/GBConversationalService";
import { Session } from "botbuilder";
import { GuaribasPackage } from "../../core.gbapp/models/GBModel";

export class KBService {

  getAnswerById(
    instanceId: number,
    answerId: number,
    cb: GBServiceCallback<GuaribasAnswer>
  ) {
    GuaribasAnswer.findAll({
      where: {
        instanceId: instanceId,
        answerId: answerId
      }
    }).then((item: GuaribasAnswer[]) => {
      cb(item[0], null);
    });
  }

  getAnswerByText(
    instanceId: number,
    text: string,
    cb: GBServiceCallback<any>
  ) {
    GuaribasQuestion.findOne({
      where: {
        instanceId: instanceId,
        content: `${text.trim()}?`
      }
    }).then((question: GuaribasQuestion) => {
      GuaribasAnswer.findAll({
        where: {
          instanceId: instanceId,
          answerId: question.answerId
        }
      }).then((answer: GuaribasAnswer[]) => {
        cb({ question: question, answer: answer[0] }, null);
      });
    });
  }


  addAnswer(obj: GuaribasAnswer, cb: GBServiceCallback<GuaribasAnswer>) {
    GuaribasAnswer.create(obj).then(item => {
      if (cb) {
        cb(item, null);
      }
    });
  }

  ask(
    instance: IGBInstance,
    what: string,
    searchScore: number,
    subjects: GuaribasSubject[],
    cb: GBServiceCallback<any>
  ) {

    // Builds search query.

    what = what.replace("?", " ");
    what = what.replace("!", " ");
    what = what.replace(".", " ");
    what = what.replace("/", " ");
    what = what.replace("\\", " ");

    if (subjects) {
      what = `${what} ${KBService.getSubjectItemsSeparatedBySpaces(
        subjects
      )}`;
    }

    // TODO: Filter by instance. what = `${what}&$filter=instanceId eq ${instanceId}`;

    // Performs search.

    var _this = this;

    if (instance.searchKey) {
      let service = new AzureSearch(
        instance.searchKey,
        instance.searchHost,
        instance.searchIndex,
        instance.searchIndexer
      );

      service.search(what, (err: any, results: any) => {
        if (results && results.length > 0) {
          // Ponders over configuration.

          if (results[0]["@search.score"] >= searchScore) {
            _this.getAnswerById(
              instance.instanceId,
              results[0].answerId,
              (answer, err) => {
                cb({ answer: answer, questionId: results[0].questionId }, null);
              }
            );
          } else {
            cb(null, null);
          }
        } else {
          cb(null, null);
        }
      });
    } else {
      this.getAnswerByText(instance.instanceId, what, (data, err) => {
        cb({ answer: data.answer, questionId: data.question.questionId }, null);
      });
    }
  }

  createGuaribasKbIndex(cb, name) {
    let _this = this;
    let schema = {
      name: name,
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
    };

    // TODO: Migrate to Azure Search.
    // this.client.createIndex(schema, function(err, schemaReturned) {

    //   let schemaIndexer = {
    //     name: _this.searchIndexer,
    //     description: 'gb',
    //     dataSourceName: 'gb', // TODO: Create it too dynamically from .env.
    //     targetIndexName: _this.searchIndex,
    //     parameters: {
    //       'maxFailedItems' : 10,
    //       'maxFailedItemsPerBatch' : 5,
    //       'base64EncodeKeys': false,
    //       'batchSize': 500
    //     }};

    //   // create/update an indexer
    //   _this.client.createIndexer(schemaIndexer, function(err, schemaIndexerReturned){
    //     cb(schemaIndexerReturned, err);
    //   });

    // });
  }

  static getFormattedSubjectItems(subjects: GuaribasSubject[]) {
    if (!subjects) return "";
    let out = [];
    subjects.forEach(subject => {
      out.push(subject.title);
    });
    return out.join(", ");
  }

  static getSubjectItemsSeparatedBySpaces(subjects: GuaribasSubject[]) {
    let out = [];
    subjects.forEach(subject => {
      out.push(subject.title);
    });
    return out.join(" ");
  }

  getSubjectItems(
    instanceId: number,
    parentId: number,
    cb: GBServiceCallback<GuaribasSubject[]>
  ) {
    var where = { parentSubjectId: parentId, instanceId: instanceId };
    GuaribasSubject.findAll({
      where: where
    })
      .then((values: GuaribasSubject[]) => {
        cb(values, null);
      })
      .error(reason => {
        cb(null, reason);
      });
  }

  getFaqBySubjectArray(from: string, subjects: any, cb) {
    let where = {
      from: from
    };

    if (subjects) {
      if (subjects[0]) {
        where["subject1"] = subjects[0].title;
      }

      if (subjects[1]) {
        where["subject2"] = subjects[1].title;
      }

      if (subjects[2]) {
        where["subject3"] = subjects[2].title;
      }

      if (subjects[3]) {
        where["subject4"] = subjects[3].title;
      }
    }
    GuaribasQuestion.findAll({
      where: where
    })
      .then((items: GuaribasQuestion[]) => {
        if (!items) items = [];
        if (items.length == 0) {
          cb([], null);
        } else {
          cb(items, null);
        }
      })
      .catch(reason => {
        if (reason.message.indexOf("no such table: IGBInstance") != -1) {
          cb([], null);
        } else {
          cb(null, reason);
          logger.trace(`GuaribasServiceError: ${reason}`);
        }
      });
  }


  importKbTabularFile(
    basedir: string,
    filename: string,
    instanceId: number,
    packageId: number,
    cb
  ) {
    var filePath = UrlJoin(basedir, filename);

    var parser = Parse(
      {
        delimiter: "\t"
      },
      function (err, data) {
        Async.eachSeries(data, function (line, callback) {
          callback();
          let subjectsText = line[0];
          var from = line[1];
          var to = line[2];
          var question = line[3];
          var answer = line[4];

          // Skip the first line.

          if (!(subjectsText === "subjects" && from == "from")) {
            let format = ".txt";

            // Extract answer from external media if any.

            if (answer.indexOf(".md") > -1) {
              let mediaFilename = UrlJoin(basedir, "..", "articles", answer);
              if (Fs.existsSync(mediaFilename)) {
                answer = Fs.readFileSync(mediaFilename, "utf8");
                format = ".md";
              } else {
                logger.trace("[GBImporter] File not found: ", mediaFilename);
                answer =
                  "Por favor, contate a administração para rever esta pergunta.";
              }
            }

            let subjectArray = subjectsText.split(".");
            let subject1: string,
              subject2: string,
              subject3: string,
              subject4: string;

            var indexer = 0;
            subjectArray.forEach(element => {
              if (indexer == 0) {
                subject1 = subjectArray[indexer].substring(0, 63);
              } else if (indexer == 1) {
                subject2 = subjectArray[indexer].substring(0, 63);
              } else if (indexer == 2) {
                subject3 = subjectArray[indexer].substring(0, 63);
              } else if (indexer == 3) {
                subject4 = subjectArray[indexer].substring(0, 63);
              }
              indexer++;
            });

            GuaribasAnswer.create({
              instanceId: instanceId,
              content: answer,
              format: format,
              packageId: packageId
            }).then(function (answer: GuaribasAnswer) {
              GuaribasQuestion.create({
                from: from,
                to: to,
                subject1: subject1,
                subject2: subject2,
                subject3: subject3,
                subject4: subject4,
                content: question,
                instanceId: instanceId,
                answerId: answer.answerId,
                packageId: packageId
              });
            });
          } else {
            logger.warn("[GBImporter] Missing header in file: ", filename);
          }
        });
      }
    );
    Fs.createReadStream(filePath, {
      encoding: "UCS-2"
    }).pipe(parser);
  }

  sendAnswer(conversationalService: IGBConversationalService, session: Session, answer: GuaribasAnswer) {

    if (answer.content.endsWith('.mp4')) {
      conversationalService.sendEvent(session, "play", {
        playerType: "video",
        data: answer.content
      });
    } else if (answer.content.length > 140) {
      let msgs = [
        "Vou te responder na tela para melhor visualização...",
        "A resposta está na tela...",
        "Veja a resposta na tela..."
      ];
      session.send(msgs);
      var html = answer.content;
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
        });
        html = marked(answer.content);
      }
      conversationalService.sendEvent(session, "play", { playerType: "markdown", data: html });
    } else {
      session.send(answer.content);
      conversationalService.sendEvent(session, "stop", null);
    }
  }


  importKbPackage(
    localPath: string,
    packageStorage: GuaribasPackage,
    instance: IGBInstance
  ) {
    this.importSubjectFile(
      packageStorage.packageId,
      UrlJoin(localPath, "subjects.json"),
      instance
    );
    let _this = this;
    setTimeout(() => {
      _this.importKbTabularDirectory(
        localPath,
        instance,
        packageStorage.packageId
      );
    }, 3000);
  }

  importKbTabularDirectory(
    localPath: string,
    instance: IGBInstance,
    packageId: number
  ) {
    let _this = this;
    Walk.files(
      UrlJoin(localPath, "tabular"),
      (basedir, filename, stat, next) => {
        if (filename.endsWith(".tsv")) {
          _this.importKbTabularFile(
            basedir,
            filename,
            instance.instanceId,
            packageId,
            (data, err) => {
              if (err) {
                logger.trace(err);
              } else {
                logger.trace("Import KB done.");
              }
            }
          );
        }
      },
      function (err) {
        if (err) logger.trace(err);
      }
    );
  }

  importSubjectFile(
    packageId: number,
    filename: string,
    instance: IGBInstance
  ) {
    var subjects = JSON.parse(Fs.readFileSync(filename, "utf8"));

    function doIt(subjects: GuaribasSubject[], parentSubjectId: number) {
      Async.eachSeries(subjects, (item, callback) => {
        let mediaFilename = item.id + ".png";
        GuaribasSubject.create({
          internalId: item.id,
          parentSubjectId: parentSubjectId,
          instanceId: instance.instanceId,
          from: item.from,
          to: item.to,
          title: item.title,
          description: item.description,
          packageId: packageId
        }).then((value: any) => {
          if (item.children) {
            doIt(item.children, value.subjectId);
          }
        });
        callback();
      });
    }
    doIt(subjects.children, null);
  }


  undeployKbFromStorage(
    instance: IGBInstance,
    packageId: number,
    cb: GBServiceCallback<any>
  ) {
    GuaribasQuestion.destroy({
      where: { instanceId: instance.instanceId, packageId: packageId }
    }).then(value => {
      GuaribasAnswer.destroy({
        where: { instanceId: instance.instanceId, packageId: packageId }
      }).then(value => {
        GuaribasSubject.destroy({
          where: { instanceId: instance.instanceId, packageId: packageId }
        }).then(value => {
          GuaribasPackage.destroy({
            where: { instanceId: instance.instanceId, packageId: packageId }
          }).then(value => {
            cb(null, null);
          });
        });
      });
    });
  }

  /**
 * Deploys a knowledge base to the storage using the .gbkb format.
 * 
 * @param localPath Path to the .gbkb folder.
 * @param cb Package instance or error info.
 */
  deployKb(core: IGBCoreService, deployer: GBDeployer, localPath: string, cb: GBServiceCallback<any>) {
    let packageType = Path.extname(localPath);
    let packageName = Path.basename(localPath);
    logger.trace("[GBDeployer] Opening package: ", packageName);
    let packageObject = JSON.parse(
      Fs.readFileSync(UrlJoin(localPath, "package.json"), "utf8")
    );

    core.loadInstance(packageObject.botId, (instance, err) => {
      deployer.deployPackageToStorage(
        instance.instanceId,
        packageName,
        (p, err) => {
          this.importKbPackage(localPath, p, instance);
          setTimeout(() => {
            cb(null, null);
          }, 8000);
        }
      );
    });
  }

}
