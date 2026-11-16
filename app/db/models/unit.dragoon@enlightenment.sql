-- unit.dragoon@enlightenment: Dragoon (mesh.dragoon_enlightenment)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.dragoon_enlightenment','Dragoon');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.dragoon_enlightenment';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('beast','quadruped','mat.hide',0,'','{"pace":0.62,"stride":30,"tint":"#c08a60"}'),
  ('saddle_cloth','box','mat.cloth',1,'beast.rider','{"y":-0.1,"z":-0.16,"w":0.38,"d":0.34,"h":0.14}'),
  ('rider','figure','mat.cloth',1,'beast.rider','{"y":-0.4,"seat":1,"attack":5,"hold_right":10,"legs":"#2a2420"}'),
  ('helmet','dome','mat.brass',0,'rider.head','{"y":-0.13,"w":0.25,"h":0.18,"segments":8}'),
  ('turban','cylinder','mat.fur',0,'rider.head','{"y":-0.13,"w":0.26,"h":0.06,"segments":8,"far":0}'),
  ('crest','box','mat.brass',0,'rider.head','{"y":0.02,"w":0.045,"d":0.24,"h":0.08,"taper":0.2}'),
  ('mane','box','mat.felt',0,'rider.head','{"y":-0.24,"z":-0.13,"w":0.06,"d":0.05,"h":0.3,"pitch":-12,"tint":"#3a3028"}'),
  ('stock','box','mat.timber',0,'rider.right_hand','{"y":-0.22,"w":0.05,"d":0.07,"h":0.32,"far":0}'),
  ('barrel','cylinder','mat.iron',0,'stock','{"y":0.26,"w":0.032,"h":0.4,"segments":5}'),
  ('spark','cone','mat.flash',0,'barrel','{"y":0.4,"w":0.08,"h":0.08,"segments":3,"flash":1,"far":0}'),
  ('flash','cone','mat.flash',0,'spark','{"y":0.26,"w":0.24,"h":0.26,"segments":6,"pitch":180,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.dragoon_enlightenment'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='unit.dragoon'), NULL, (SELECT id FROM def WHERE key='age.enlightenment'), (SELECT id FROM def WHERE key='mesh.dragoon_enlightenment'), (SELECT id FROM def WHERE key='mat.cloth'), 100);
