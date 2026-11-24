-- bld.defensive_line@automation: Sensor barrier (mesh.defensive_line_automation)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.defensive_line_automation','Sensor barrier');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.defensive_line_automation';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('plinth','box','mat.white_composite',0,'','{"w":1.0,"d":0.6,"h":0.16}'),
  ('panels','box','mat.composite_panel',0,'','{"y":0.16,"w":1.0,"d":0.24,"h":1.15}'),
  ('strip','box','mat.light_strip',0,'','{"y":0.9,"z":0.125,"w":0.94,"d":0.02,"h":0.05}'),
  ('cap','box','mat.cloth',1,'','{"y":1.31,"w":0.98,"d":0.3,"h":0.07}'),
  ('sensor_post','cylinder','mat.white_metal',0,'','{"x":0.46,"y":0.16,"w":0.1,"h":1.45,"segments":6}'),
  ('sensor','dome','mat.glow',0,'','{"x":0.46,"y":1.6,"w":0.16,"h":0.1,"segments":6}'),
  ('traps','cone','mat.white_composite',0,'','{"x":-0.25,"z":0.42,"w":0.24,"h":0.3,"segments":4,"repeat":2,"dx":0.5,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.defensive_line_automation'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.defensive_line'), NULL, (SELECT id FROM def WHERE key='age.automation'), (SELECT id FROM def WHERE key='mesh.defensive_line_automation'), (SELECT id FROM def WHERE key='mat.white_composite'), 100);
