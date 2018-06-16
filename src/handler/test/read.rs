use responder::StaticFile;
use validator::AlphaNum;

pub enum ReadHtml {}

#[get("/read.cgi/<_board>/<_key>")]
fn get<'r>(_board: AlphaNum, _key: u64, html: &'r StaticFile<ReadHtml>) -> &'r StaticFile<ReadHtml> {
    html
}
