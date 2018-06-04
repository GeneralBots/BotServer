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

"use strict";

import {
  DataTypes,
  DataTypeUUIDv4,
  DataTypeDate,
  DataTypeDecimal
} from "sequelize";

import {
  Sequelize,
  Table,
  Column,
  Model,
  HasMany,
  BelongsTo,
  BelongsToMany,
  Length,
  ForeignKey,
  CreatedAt,
  UpdatedAt,
  DataType,
  IsUUID,
  PrimaryKey,
  AutoIncrement
} from "sequelize-typescript";

import { GuaribasUser } from "../../security.gblib/models";
import { GuaribasInstance, GuaribasPackage } from "../../core.gbapp/models/GBModel";

@Table
export class GuaribasSubject extends Model<GuaribasSubject> {
  @PrimaryKey
  @AutoIncrement
  @Column
  subjectId: number;

  @Column internalId: string;

  @Column title: string;

  @Column description: string;

  @Column from: string;

  @Column to: string;

  @ForeignKey(() => GuaribasSubject)
  @Column
  parentSubjectId: number;

  @BelongsTo(() => GuaribasSubject, "parentSubjectId")
  parentSubject: GuaribasSubject;

  @HasMany(() => GuaribasSubject, { foreignKey: "parentSubjectId" })
  childrenSubjects: GuaribasSubject[];

  @ForeignKey(() => GuaribasInstance)
  @Column
  instanceId: number;

  @BelongsTo(() => GuaribasInstance)
  instance: GuaribasInstance;

  @ForeignKey(() => GuaribasUser)
  @Column
  responsibleUserId: number;

  @BelongsTo(() => GuaribasUser)
  responsibleUser: GuaribasUser;

  @ForeignKey(() => GuaribasPackage)
  @Column
  packageId: number;

  @BelongsTo(() => GuaribasPackage)
  package: GuaribasPackage;
}

@Table
export class GuaribasQuestion extends Model<GuaribasQuestion> {
  @PrimaryKey
  @AutoIncrement
  @Column
  questionId: number;

  @Column(DataType.STRING(64))
  @Column
  subject1: string;

  @Column(DataType.STRING(64))
  @Column
  subject2: string;

  @Column(DataType.STRING(64))
  @Column
  subject3: string;

  @Column(DataType.STRING(64))
  @Column
  subject4: string;

  @Column(DataType.STRING(1024))
  @Column
  keywords: string;

  @Column(DataType.STRING(512))
  from: string;

  @Column(DataType.STRING(512))
  to: string;

  @Column(DataType.TEXT)
  content: string;

  @Column
  @CreatedAt
  createdAt: Date;

  @Column
  @UpdatedAt
  updatedAt: Date;

  @ForeignKey(() => GuaribasAnswer)
  @Column
  answerId: number;

  @BelongsTo(() => GuaribasInstance)
  instance: GuaribasInstance;

  @ForeignKey(() => GuaribasInstance)
  @Column
  instanceId: number;

  @ForeignKey(() => GuaribasPackage)
  @Column
  packageId: number;

  @BelongsTo(() => GuaribasPackage)
  package: GuaribasPackage;
}

@Table
export class GuaribasAnswer extends Model<GuaribasAnswer> {
  @PrimaryKey
  @AutoIncrement
  @Column
  answerId: number;

  @Length({ min: 0, max: 512 })
  @Column
  media: string;

  @Length({ min: 0, max: 12 })
  @Column
  format: string;

  @Column(DataType.TEXT)
  content: string;

  @Column
  @CreatedAt
  createdAt: Date;

  @Column
  @UpdatedAt
  updatedAt: Date;

  @HasMany(() => GuaribasQuestion)
  questions: GuaribasQuestion[];

  @ForeignKey(() => GuaribasInstance)
  @Column
  instanceId: number;

  @ForeignKey(() => GuaribasPackage)
  @Column
  packageId: number;

  @BelongsTo(() => GuaribasPackage)
  package: GuaribasPackage;

}
