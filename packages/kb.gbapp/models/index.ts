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
 * @fileoverview General Bots server core.
 */

'use strict';

import {
  AutoIncrement,
  BelongsTo,
  Column,
  CreatedAt,
  DataType,
  ForeignKey,
  HasMany,
  HasOne,
  IsUUID,
  Length,
  Model,
  PrimaryKey,
  Sequelize,
  Table,
  UpdatedAt
} from 'sequelize-typescript';

import { GuaribasInstance, GuaribasPackage } from '../../core.gbapp/models/GBModel.js';
import { GuaribasUser } from '../../security.gbapp/models/index.js';

/**
 * Subjects to group the pair of questions and answers.
 */
@Table
export class GuaribasSubject extends Model<GuaribasSubject> {
  @PrimaryKey
  @AutoIncrement
  @Column(DataType.INTEGER)
  declare subjectId: number;

  @Column(DataType.STRING(255))
  declare internalId: string;

  declare title: string;

  @Column(DataType.STRING(512))
  declare description: string;

  @Column(DataType.STRING(255))
  declare from: string;

  @Column(DataType.STRING(255))
  declare to: string;

  @ForeignKey(() => GuaribasSubject)
  @Column(DataType.INTEGER)
  declare parentSubjectId: number;

  @BelongsTo(() => GuaribasSubject, 'parentSubjectId')
  parentSubject: GuaribasSubject;

  @HasMany(() => GuaribasSubject, { foreignKey: 'parentSubjectId' })
  declare childrenSubjects: GuaribasSubject[];

  @ForeignKey(() => GuaribasInstance)
  @Column(DataType.INTEGER)
  declare instanceId: number;

  @BelongsTo(() => GuaribasInstance)
  declare instance: GuaribasInstance;

  @ForeignKey(() => GuaribasUser)
  @Column(DataType.INTEGER)
  declare responsibleUserId: number;

  @BelongsTo(() => GuaribasUser)
  declare responsibleUser: GuaribasUser;

  @ForeignKey(() => GuaribasPackage)
  @Column(DataType.INTEGER)
  declare packageId: number;

  @BelongsTo(() => GuaribasPackage)
  declare package: GuaribasPackage;
}

/**
 * A question and its metadata.
 */
@Table
export class GuaribasQuestion extends Model<GuaribasQuestion> {
  @PrimaryKey
  @AutoIncrement
  @Column(DataType.INTEGER)
  declare questionId: number;

  @Column(DataType.STRING(64))
  declare subject1: string;

  @Column(DataType.STRING(64))
  declare subject2: string;

  @Column(DataType.STRING(64))
  declare subject3: string;

  @Column(DataType.STRING(64))
  declare subject4: string;

  @Column(DataType.STRING(1024))
  declare keywords: string;

  @Column(DataType.BOOLEAN)
  declare skipIndex: boolean;

  @Column(DataType.STRING(512))
  declare from: string;

  @Column(DataType.STRING(512))
  declare to: string;

  @Column(DataType.TEXT)
  declare content: string;

  @Column(DataType.DATE)
  @CreatedAt
  declare createdAt: Date;

  @Column(DataType.DATE)
  @UpdatedAt
  declare updatedAt: Date;

  //tslint:disable-next-line:no-use-before-declare
  @ForeignKey(() => GuaribasAnswer)
  @Column(DataType.INTEGER)
  declare answerId: number;

  @BelongsTo(() => GuaribasInstance)
  declare instance: GuaribasInstance;

  @ForeignKey(() => GuaribasInstance)
  @Column(DataType.INTEGER)
  declare instanceId: number;

  @ForeignKey(() => GuaribasPackage)
  @Column(DataType.INTEGER)
  declare packageId: number;

  @BelongsTo(() => GuaribasPackage)
  package: GuaribasPackage;
}

/**
 * An answer and its metadata.
 */
@Table
export class GuaribasAnswer extends Model<GuaribasAnswer> {
  @PrimaryKey
  @AutoIncrement
  @Column(DataType.INTEGER)
  declare answerId: number;

  @Length({ min: 0, max: 512 })
  @Column(DataType.STRING(512))
  declare media: string;

  @Length({ min: 0, max: 12 })
  @Column(DataType.STRING(12))
  declare format: string;

  @Column(DataType.TEXT)
  declare content: string;

  @Column(DataType.DATE)
  @CreatedAt
  declare createdAt: Date;

  @Column(DataType.DATE)
  @UpdatedAt
  declare updatedAt: Date;

  @HasMany(() => GuaribasQuestion)
  declare questions: GuaribasQuestion[];

  @HasOne(() => GuaribasQuestion)
  declare prev: GuaribasQuestion;

  @HasOne(() => GuaribasQuestion)
  declare next: GuaribasQuestion;

  @ForeignKey(() => GuaribasQuestion)
  @Column(DataType.INTEGER)
  declare nextId: number;

  @ForeignKey(() => GuaribasQuestion)
  @Column(DataType.INTEGER)
  declare prevId: number;

  @ForeignKey(() => GuaribasInstance)
  @Column(DataType.INTEGER)
  declare instanceId: number;

  @ForeignKey(() => GuaribasPackage)
  @Column(DataType.INTEGER)
  declare packageId: number;

  @BelongsTo(() => GuaribasPackage)
  declare package: GuaribasPackage;
}
