"""Only generated test fixtures inside an explicitly isolated profile."""
import os,zipfile,base64
from pathlib import Path
root=Path(__file__).resolve().parents[1]/'.test-runtime'/'materials-ui'
assert Path(os.environ['TEMP']).resolve()==(root/'temp').resolve()
out=root/'workspace'
out.mkdir(parents=True,exist_ok=True)
(out/'交流要点.txt').write_text('合成资料：专家希望了解随访终点的证据。需要核对适用人群，下一次交流讨论研究合作条件。',encoding='utf-8')
(out/'暂不支持.custom').write_bytes(b'synthetic-unsupported-content')
png=base64.b64decode('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+j5RkAAAAASUVORK5CYII=')
(out/'示意图.png').write_bytes(png)
# An image-only, valid PDF: there is no extractable text to fake scan recognition.
objects=[b'<< /Type /Catalog /Pages 2 0 R >>',b'<< /Type /Pages /Kids [3 0 R] /Count 1 >>',b'<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 400] /Resources << /XObject << /Im0 4 0 R >> >> /Contents 5 0 R >>',b'<< /Type /XObject /Subtype /Image /Width 1 /Height 1 /ColorSpace /DeviceRGB /BitsPerComponent 8 /Length 3 >>\nstream\n\x60\x80\x90\nendstream',b'<< /Length 28 >>\nstream\nq 200 0 0 200 50 100 cm /Im0 Do Q\nendstream']
objects[4]=b'<< /Length 33 >>\nstream\nq 200 0 0 200 50 100 cm /Im0 Do Q\nendstream'
data=bytearray(b'%PDF-1.4\n');offsets=[0]
for i,obj in enumerate(objects,1):
    offsets.append(len(data));data.extend(str(i).encode()+b' 0 obj\n'+obj+b'\nendobj\n')
xref=len(data);data.extend(b'xref\n0 6\n0000000000 65535 f \n')
for pos in offsets[1:]:data.extend(('%010d 00000 n \n'%pos).encode())
data.extend(('trailer\n<< /Size 6 /Root 1 0 R >>\nstartxref\n%d\n%%%%EOF'%xref).encode())
(out/'扫描资料.pdf').write_bytes(data)
def office(filename,kind,part,xml):
    with zipfile.ZipFile(out/filename,'w',zipfile.ZIP_DEFLATED) as z:
        z.writestr('[Content_Types].xml',f'<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="xml" ContentType="application/xml"/><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Override PartName="/{part}" ContentType="{kind}"/></Types>')
        z.writestr('_rels/.rels',f'<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="{part}"/></Relationships>')
        z.writestr(part,xml)
office('访谈材料.docx','application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml','word/document.xml','<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p><w:r><w:t>合成访谈材料：随访证据需求。</w:t></w:r></w:p></w:body></w:document>')
office('交流演示.pptx','application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml','ppt/presentation.xml','<p:presentation xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"><p:sldIdLst/></p:presentation>')
office('研究表格.xlsx','application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml','xl/workbook.xml','<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheets/></workbook>')
print('Generated 7 synthetic fixtures; no real files read.')
