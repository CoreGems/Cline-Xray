import os
from dotenv import load_dotenv
from jira import JIRA

# Load environment variables from .env file
load_dotenv()

jira = JIRA(
    os.getenv('JIRA_URL'),
    basic_auth=(os.getenv('JIRA_EMAIL'), os.getenv('JIRA_API_TOKEN'))
)

issue = jira.issue('ADMIN-1808')
print(issue.fields.summary)