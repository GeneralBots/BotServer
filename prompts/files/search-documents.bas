PARAM query AS STRING
PARAM location AS STRING OPTIONAL
PARAM file_type AS STRING OPTIONAL
PARAM date_range AS ARRAY OPTIONAL

search_params = {
  "query": query
}

IF location IS NOT NULL THEN
  search_params["location"] = location
END IF

IF file_type IS NOT NULL THEN
  search_params["file_type"] = file_type
END IF

IF date_range IS NOT NULL THEN
  search_params["created_after"] = date_range[0]
  search_params["created_before"] = date_range[1]
END IF

results = CALL "/files/search", search_params

IF LEN(results) = 0 THEN
  RETURN "No documents found matching your criteria."
END IF

# Format results for display
formatted_results = "Found " + LEN(results) + " documents:\n\n"
FOR EACH doc IN results
  formatted_results = formatted_results + "- " + doc.name + " (" + FORMAT_DATE(doc.modified) + ")\n"
  formatted_results = formatted_results + "  Location: " + doc.path + "\n"
NEXT

RETURN formatted_results
