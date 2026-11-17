"""Generate/read back a feature-rich reference from the pinned CMI checkout.

uv run --locked --group test python tests/fixtures/generate_features.py /tmp/cmi-docx-reference /tmp/rust.docx
The second argument is optional; when present, compare semantic readback.
"""
import asyncio
import io
import pathlib
import subprocess
import sys

root=pathlib.Path(sys.argv[1]).resolve()
assert subprocess.check_output(['git','-C',str(root),'rev-parse','HEAD'],text=True).strip()=='3449b942034129ad74a28aeb253361035806e385'
sys.path.insert(0,str(root/'src'))
from cmi_docx.declarative import *
from docx import Document as ReadDocument
from docx.enum.text import WD_ALIGN_PARAGRAPH as A
from docx.shared import Inches


def cell(text):return TableCell(children=[Paragraph(text=text)])

async def reference():
    ps=ParagraphStyleDefinition(name='ReportBody',base_style='Normal',next_paragraph_style='Normal',font='Arial',font_size=11,bold=False,italic=True,underline=False,color=(12,34,56),alignment=A.LEFT,spacing_before=6,spacing_after=4,line_spacing=1.2,left_indent=.25,right_indent=.5,first_line_indent=-.125,keep_together=True,keep_with_next=False,page_break_before=False,widow_control=True)
    ts=TableStyleDefinition(name='ReportTable',base_style='Normal Table',whole_table=TableSectionFormat(font='Calibri',font_size=10),first_row=TableSectionFormat(bold=True,color=(255,255,255),background=(0,32,96)))
    p=Paragraph(style='ReportBody',alignment=A.JUSTIFY,spacing_before=12,spacing_after=8,line_spacing=1.5,left_indent=18,right_indent=9,first_line_indent=-6,keep_together=True,keep_with_next=False,page_break_before=False,widow_control=False,comment_text='Paragraph note',comment_author='Paragraph Author',children=[TextRun(text='Formatted ',bold=True,italic=False,underline=True,font='Arial',size=14,color=(1,2,3),strike=True,all_caps=True,small_caps=False,comment_text='Run note'),TextRun(text='sup',superscript=True),TextRun(text='sub',subscript=True),Tab(),TextRun(text='after tab'),Break(),TextRun(text='after line'),Break(type='page'),Break(type='column')])
    image=pathlib.Path(__file__).parent/'images/sample.png'
    table=Table(column_widths=[1000,2000,3000],style='ReportTable',borders=[TableBorder(side='insideH',sz=8,color=(128,0,0))],rows=[TableRow(children=[TableCell(children=[Paragraph(text='merged')],grid_span=2,vmerge='restart',background_color=(255,238,221),vertical_alignment='center',borders=[CellBorder(side='top',sz=4,color=(16,32,48),val='dashed')]),TableCell(children=[Table(rows=[TableRow(children=[cell('nested')])]),Paragraph(text='after nested')])]),TableRow(children=[TableCell(children=[Paragraph(text='continuation')],grid_span=2,vmerge='continue'),cell('tail')]),TableRow(children=[cell('short'),cell('row')]),TableRow(children=[cell('hidden')],condition=lambda:False)])
    doc=Document(sections=[Section(children=[Paragraph(text='Feature parity report',heading=1),p,Paragraph(children=[ImageRun(data=image,transformation={'width':72})]),table],properties=SectionProperties(page_orientation='portrait',page_margins={'top':Inches(1),'bottom':Inches(1),'left':Inches(1.25),'right':Inches(1.25)}),headers={'default':Header(children=[Paragraph(text='Default header')]),'first':Header(children=[Paragraph(text='First header')]),'even':Header(children=[Paragraph(text='Even header')])},footers={'default':Footer(children=[Paragraph(text='Footer')])}),Section(children=[Paragraph(text='Landscape section')],properties=SectionProperties(page_orientation='landscape'))],creator='Rust Author',title='Parity report',subject='Testing',description='Description',comments='Core comments',keywords='rust, docx',category='Reports',version='1.0',comment_author='Default Author',styles=[ps,ts])
    return await doc.to_docx()


def summary(doc):
    def paragraph(p):
        fmt=p.paragraph_format
        return {'text':p.text,'style':p.style.name,'format':[getattr(fmt,n) for n in ('alignment','space_before','space_after','line_spacing','left_indent','right_indent','first_line_indent','keep_together','keep_with_next','page_break_before','widow_control')], 'runs':[(r.text,r.bold,r.italic,r.underline,r.font.name,r.font.size,str(r.font.color.rgb),r.font.superscript,r.font.subscript,r.font.strike,r.font.all_caps,r.font.small_caps) for r in p.runs]}
    def table(t):
        return {'widths':[c.width for c in t.columns],'autofit':t.autofit,'style':t.style.name,'cells':[[{'text':c.text,'span':c._tc.grid_span,'merge':c._tc.vMerge,'align':c.vertical_alignment,'tables':[table(n) for n in c.tables]} for c in row.cells] for row in t.rows]}
    def style(name):
        s=doc.styles[name];pf=s.paragraph_format
        return [s.name,s.base_style.name,s.font.name,s.font.size,s.font.bold,s.font.italic,s.font.underline,str(s.font.color.rgb),pf.left_indent,pf.right_indent,pf.first_line_indent,pf.space_before,pf.space_after,pf.line_spacing,pf.keep_together,pf.keep_with_next,pf.page_break_before,pf.widow_control]
    cp=doc.core_properties
    return {'paragraphs':[paragraph(p) for p in doc.paragraphs], 'tables':[table(t) for t in doc.tables], 'sections':[(s.page_width,s.page_height,s.top_margin,s.bottom_margin,s.left_margin,s.right_margin,s.orientation,[p.text for p in s.header.paragraphs],[p.text for p in s.first_page_header.paragraphs],[p.text for p in s.even_page_header.paragraphs],[p.text for p in s.footer.paragraphs]) for s in doc.sections], 'images':[(s.width,s.height) for s in doc.inline_shapes], 'comments':[(c.author,c.text) for c in doc.comments], 'styles':[style('ReportBody')], 'metadata':[getattr(cp,n) for n in ('author','title','subject','comments','keywords','category','version')]}


async def main():
    doc=await reference()
    target=pathlib.Path(__file__).parent/'features.docx'
    doc.save(target)
    if len(sys.argv)>2:
        import json
        expected=summary(doc);actual=summary(ReadDocument(sys.argv[2]))
        different=[key for key in expected if actual[key]!=expected[key]]
        for key in different:
            print(key, '\nExpected:',json.dumps(expected[key],ensure_ascii=False),'\nActual:',json.dumps(actual[key],ensure_ascii=False))
        assert not different, f'Differences: {different}'
        print('Python semantic parity: paragraphs, runs, styles, tables, sections, images, comments and metadata match.')

asyncio.run(main())
