## Myers' Diff Algorithm For GraphQL Queries
This is an example implementation of using the Myers' diff algorithm to get the differences between the expected and actual queries. This implementation is not 100% as there are
quite a few cases in which it would break. The cases it does not work for and will need to be enhanced to handle are: 
- when the expected and actual query are the same but ordered differently (this can be easily fixed by ordering all the fields alphabetically)
- different variable names for expected and actual query which can be fixed by doing a replacement of variable names for one of the queries
- inline fragments which can be handled by possibly removing the fragment name and replacing it with the fragment somehow -> probably the most annoying case to handle