

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
